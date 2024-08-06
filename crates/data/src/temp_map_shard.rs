use crate::data_shard::DataShard;
use crate::map_shard::MapShard;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;
use crate::temp_offset_types::TempOffsetTypes;

#[derive(Debug)]
pub struct TempMapShard {
    folder: PathBuf,
    prefix: String,
    max_offsets: TempOffsetTypes,
    pub temp_shards: RwLock<HashMap<String, DataShard>>,
}

impl TempMapShard {
    pub fn new(folder: PathBuf, max_offsets: TempOffsetTypes, prefix: &str) -> Self {
        TempMapShard {
            folder,
            prefix: prefix.to_string(),
            max_offsets,
            temp_shards: RwLock::new(HashMap::new()),
        }
    }

    fn create_shard(&self) -> DataShard {
        let shard_path = self.folder.join(format!(
            "{}{}",
            self.prefix.clone(),
            Uuid::new_v4().to_string()
        ));
        DataShard::new(shard_path, self.max_offsets.get_real_offset(), None)
    }

    pub fn insert_row(&self, data: Vec<u8>) {
        let find_usable_shard = {
            let instant = std::time::Instant::now();
            let mut shards = self.temp_shards.read().unwrap();
            //println!("Took to acquire temp_shards lock : {:.5?}", instant.elapsed());
            shards.iter()
                .find(|i| i.1.has_space())
                .map(|i| i.0.clone())
        };

        let shard_key = match find_usable_shard {
            None => {
                let mut shards = self.temp_shards.write().unwrap();
                let shard = self.create_shard();
                let shard_id = shard.get_id();
                shards.insert(shard_id.clone(), shard);
                shard_id
            }
            Some(shard) => shard,
        };

        {
            let mut shards = self.temp_shards.read().unwrap();
            shards.get(&shard_key).unwrap().insert_item(data).unwrap();
        }
    }

    pub fn reconcile(&self, master: &mut MapShard) {
        let mut shards = self.temp_shards.write().unwrap();
        for (_id, shard) in shards.iter() {
            let header = shard.header.read().unwrap();
            header.offsets.iter().for_each(|&item_offset| {
                let binary_item = shard.read_item(item_offset).unwrap();
                master
                    .current_master_shard
                    .insert_item(binary_item)
                    .unwrap();
            });
        }
        shards.clear();
    }
}
