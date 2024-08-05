use crate::data_shard::DataShard;
use crate::map_shard::MapShard;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct TempMapShard {
    folder: PathBuf,
    prefix: String,
    max_offsets: Option<u64>,
    pub temp_shards: RwLock<HashMap<String, DataShard>>,
}

impl TempMapShard {
    pub fn new(folder: PathBuf, max_offsets: Option<u64>, prefix: &str) -> Self {
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
        DataShard::new(shard_path, self.max_offsets.clone(), None)
    }

    pub fn insert_row(&self, data: Vec<u8>) {
        let mut shards = self.temp_shards.write().unwrap();
        let find_usable_shard = shards.iter().find(|i| i.1.has_space());
        let shard_key = match find_usable_shard {
            None => {
                let shard = self.create_shard();
                let shard_id = shard.get_id();
                shards.insert(shard_id.clone(), shard);
                shard_id
            }
            Some(shard) => shard.0.clone(),
        };

        shards.get(&shard_key).unwrap().insert_item(data).unwrap();
    }

    pub fn reconcile(&self, master: &MapShard) {
        let mut shards = self.temp_shards.write().unwrap();
        let master_writer = master.current_master_shard.write().unwrap();
        for (_id, shard) in shards.iter() {
            let header = shard.header.read().unwrap();
            header.offsets.iter().for_each(|&item_offset| {
                let binary_item = shard.read_item(item_offset).unwrap();
                master_writer.insert_item(binary_item).unwrap();
            });
        }
        shards.clear();
    }
}
