use crate::data_shard::DataShard;
use crate::map_shard::MapShard;
use crate::temp_offset_types::TempOffsetTypes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::RwLockWriteGuard;
use uuid::Uuid;

#[derive(Debug)]
pub struct TempMapShard {
    folder: PathBuf,
    prefix: String,
    max_offsets: TempOffsetTypes,
    parent_shard: Arc<RwLock<MapShard>>,
    pub temp_shards: RwLock<Vec<DataShard>>,
}

impl TempMapShard {
    pub fn new(
        folder: PathBuf,
        parent_shard: Arc<RwLock<MapShard>>,
        max_offsets: TempOffsetTypes,
        prefix: &str,
    ) -> Self {
        TempMapShard {
            parent_shard,
            folder,
            prefix: prefix.to_string(),
            max_offsets,
            temp_shards: RwLock::new(vec![]),
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
            let mut shards = self.temp_shards.read().unwrap();
            shards.iter().position(|i| i.has_space())
        };

        let shard_key = match find_usable_shard {
            None => {
                {
                    // Reconcile last

                    self.reconcile_specific(None);
                }
                let mut shards = self.temp_shards.write().unwrap();
                let shard = self.create_shard();
                shards.push(shard);
                shards.len() - 1
            }
            Some(shard) => shard,
        };

        {
            let mut shards = self.temp_shards.read().unwrap();
            shards.get(shard_key).unwrap().insert_item(data).unwrap();
        }
    }

    pub fn reconcile_specific(&self, shard_position: Option<usize>) {
        let pos = {
            let reader = self.temp_shards.read().unwrap();
            let index = shard_position.or_else(|| {
                if reader.is_empty() {
                    None
                } else {
                    Some(reader.len() - 1)
                }
            });

            let index = match index {
                Some(idx) => idx,
                None => return,
            };

            match reader.get(index) {
                None => return,
                Some(shard) => {
                    let offsets = {
                        let reader = shard.header.read().unwrap();
                        reader.offsets.to_vec()
                    };

                    {
                        let mut writer = self.parent_shard.write().unwrap();
                        for item_offset in offsets {
                            let binary_item = shard.read_item(item_offset).unwrap();
                            writer.insert_row(binary_item)
                        }
                    }
                }
            };
            index
        };

        {
            let mut writer = self.temp_shards.write().unwrap();
            writer.remove(pos);
        }
    }
}
