use crate::data_shard::DataShard;
use crate::map_shard::MapShard;
use crate::temp_offset_types::TempOffsetTypes;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
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

        let shard_index = match find_usable_shard {
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
            shards.get(shard_index).unwrap().insert_item(data).unwrap();
        }
    }

    fn get_reconciliation_data(shard: &DataShard) -> (&DataShard, Range<i64>) {
        let indexes = {
            let header_reader = shard.header.read().unwrap();
            let last_index = header_reader.get_last_offset_index();
            if last_index < 0 {
                0..0
            } else {
                0..(last_index + 1)
            }
        };

        (shard, indexes)
    }

    fn reconcile(from: &DataShard, target: &mut MapShard) {
        let (shard, indexes) = Self::get_reconciliation_data(from);
        for item_index in indexes {
            let binary_item = shard.read_item_from_index(item_index as usize).unwrap();
            target.insert_row(binary_item)
        }
    }

    pub fn reconcile_all(&self) {
        let mut parent_writer = self.parent_shard.write().unwrap();
        let mut temp_shards_writer = self.temp_shards.write().unwrap();

        for from_shard in temp_shards_writer.iter() {
            Self::reconcile(from_shard, &mut parent_writer);
        }

        temp_shards_writer.clear()
    }

    pub fn reconcile_specific(&self, shard_position: Option<usize>) {
        let pos = {
            let reader = self.temp_shards.read().unwrap();
            let index = shard_position.or_else(|| reader.len().checked_sub(1));

            if let Some(index) = index {
                if let Some(shard) = reader.get(index) {
                    let mut parent_shard = self.parent_shard.write().unwrap();
                    Self::reconcile(shard, &mut parent_shard);
                    index
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        self.temp_shards.write().unwrap().remove(pos);
    }
}

#[cfg(test)]
mod test {
    use crate::map_shard::MapShard;
    use crate::temp_map_shard::TempMapShard;
    use crate::temp_offset_types::TempOffsetTypes;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    pub async fn test_temp_shard() {
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from("./test_cases/data"));

        if !data_path.exists() {
            std::fs::create_dir(data_path.clone().clone()).unwrap();
        }

        let parent_shard = Arc::new(RwLock::new(MapShard::new(
            data_path.clone(),
            "localdata_",
            None,
        )));

        let shard = TempMapShard::new(
            data_path.clone(),
            parent_shard.clone(),
            TempOffsetTypes::Custom(Some(2)),
            "tempdata_",
        );

        shard.insert_row("0:Hello world".as_bytes().to_vec());

        let curr_shard_id = {
            let reader = shard.temp_shards.read().unwrap();
            assert_eq!(reader.len(), 1);
            reader.first().unwrap().get_id().clone()
        };
        // It has still not be reconciled, therefore parent doesn't contain items
        let parent_items_len = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .header
            .read()
            .unwrap()
            .get_last_offset_index();
        assert_eq!(parent_items_len, -1);

        let does_shard_still_exist = shard
            .temp_shards
            .read()
            .unwrap()
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(does_shard_still_exist);

        shard.insert_row("1:Hello Cats".as_bytes().to_vec());
        // Should reconcile automatically because the tempshard only supports 2 items per shard.
        shard.insert_row("2:Hello Dogs".as_bytes().to_vec());

        // If it reconciled, it doesn't exist anymore.
        let does_shard_still_exist = shard
            .temp_shards
            .read()
            .unwrap()
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(!does_shard_still_exist);

        // There should still be a shard available which should contain "2:Hello Dogs". This one hasn't been reconciled yet.
        assert_eq!(shard.temp_shards.read().unwrap().len(), 1);
        let item = shard
            .temp_shards
            .read()
            .unwrap()
            .first()
            .unwrap()
            .read_item_from_index(0)
            .unwrap();
        assert_eq!("2:Hello Dogs".as_bytes().to_vec(), item);

        // Now that's reconciled. Parent should have the two records inserted.
        let parent_items_len = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .header
            .read()
            .unwrap()
            .get_last_offset_index();
        assert_eq!(parent_items_len, 1);

        let parent_item_1 = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .read_item_from_index(0)
            .unwrap();
        let parent_item_2 = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .read_item_from_index(1)
            .unwrap();
        assert_eq!("0:Hello world".as_bytes().to_vec(), parent_item_1);
        assert_eq!("1:Hello Cats".as_bytes().to_vec(), parent_item_2);

        std::fs::remove_dir_all(data_path).unwrap()
    }
}
