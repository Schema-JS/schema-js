use crate::errors::ShardErrors;
use crate::fdm::FileDescriptorManager;
use crate::shard::{AvailableSpace, Shard, ShardConfig};
use crate::utils::fs::list_files_with_prefix;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct MapShard<S: Shard<Opts>, Opts: ShardConfig> {
    pub current_master_shard: S,
    pub past_master_shards: RwLock<IndexMap<String, S>>,
    pub shard_prefix: String,
    pub shards_folder: PathBuf,
    config: Opts,
    fdm: Arc<FileDescriptorManager>,
}

impl<S: Shard<Opts>, Opts: ShardConfig> MapShard<S, Opts> {
    pub fn new<P: AsRef<Path> + Clone>(
        shards_folder: P,
        shard_prefix: &str,
        config: Opts,
        fdm: Arc<FileDescriptorManager>,
    ) -> Self {
        let shards_folder = shards_folder.as_ref().to_path_buf();
        let shard_files = list_files_with_prefix(&shards_folder, shard_prefix).unwrap();
        let mut sorted_files: Vec<(usize, String, PathBuf)> = Vec::new();

        for path in shard_files {
            let val = Self::extract_shard_signature(path);
            if let Some(val) = val {
                sorted_files.push(val);
            }
        }

        sorted_files.sort_by_key(|&(number, _, _)| number);

        let maybe_new_shard_id = Uuid::new_v4();
        let current_master_shard = sorted_files
            .last()
            .map(|&(_, _, ref path)| path.clone())
            .unwrap_or_else(|| {
                shards_folder.join(Self::generate_shard_name(
                    shard_prefix,
                    maybe_new_shard_id,
                    0,
                ))
            });

        let mut past_master_shards = IndexMap::new();

        for &(_, ref uuid, ref path) in &sorted_files {
            if path != &current_master_shard {
                past_master_shards.insert(
                    uuid.clone(),
                    S::new(
                        path.clone(),
                        config.clone(),
                        Some(Uuid::parse_str(uuid).unwrap()),
                        fdm.clone(),
                    ),
                );
            }
        }

        MapShard {
            current_master_shard: S::new(
                current_master_shard,
                config.clone(),
                Some(maybe_new_shard_id),
                fdm.clone(),
            ),
            past_master_shards: RwLock::new(past_master_shards),
            shard_prefix: shard_prefix.to_string(),
            shards_folder,
            config,
            fdm,
        }
    }

    fn generate_shard_name(shard_prefix: &str, maybe_new_shard_id: Uuid, number: usize) -> String {
        format!(
            "{}{}_{}.data",
            shard_prefix,
            maybe_new_shard_id.to_string(),
            number
        )
    }

    pub fn extract_shard_signature(path: PathBuf) -> Option<(usize, String, PathBuf)> {
        // Extract the filename from the path
        let file_name = path.file_name()?.to_string_lossy();

        // Split the filename by underscore and ensure we have at least three parts
        let parts: Vec<&str> = file_name.split('.').next()?.split('_').collect();
        if parts.len() < 3 {
            return None;
        }

        // Try parsing the last part as the number and extract the uuid from the second-to-last part
        let number = parts.last()?.parse::<usize>().ok()?;
        let uuid = parts.get(parts.len() - 2)?.to_string();

        // Return the tuple containing (number, uuid, path)
        Some((number, uuid, path))
    }

    pub fn insert_rows(&mut self, data: &[&[u8]]) -> usize {
        self.raw_insert_rows(data, false)
    }

    pub fn raw_insert_rows(&mut self, data: &[&[u8]], create_new_shard: bool) -> usize {
        if create_new_shard {
            let (shard_number, _, _) =
                Self::extract_shard_signature(self.current_master_shard.get_path().clone())
                    .unwrap();
            let new_shard_number = shard_number + 1;

            let shard = {
                let shard_id = Uuid::new_v4();
                let shard_path = self.shards_folder.clone().join(Self::generate_shard_name(
                    self.shard_prefix.as_str(),
                    shard_id,
                    new_shard_number,
                ));

                S::new(
                    shard_path,
                    self.config.clone(),
                    Some(shard_id),
                    self.fdm.clone(),
                )
            };

            // Add to past master
            {
                let old_master = std::mem::replace(&mut self.current_master_shard, shard);
                let mut past_ms_writer = self.past_master_shards.write();
                let (_, shard_id, _) =
                    Self::extract_shard_signature(old_master.get_path()).unwrap();
                past_ms_writer.insert(shard_id, old_master);
            }
        }

        let available_space_in_master = self.current_master_shard.available_space();

        if let AvailableSpace::Fixed(size) = available_space_in_master {
            if size == 0 {
                return self.raw_insert_rows(data, true);
            }
        }

        let up_to = match available_space_in_master {
            AvailableSpace::Fixed(size) => std::cmp::min(size, data.len()),
            AvailableSpace::Unlimited => data.len(),
        };

        let insert_data = &data[0..up_to];

        let items_pos = {
            let local_index = self.current_master_shard.insert_item(insert_data).unwrap();
            let breaking_point = self.breaking_point();
            match breaking_point {
                None => local_index as usize,
                Some(breaking_point) => {
                    let reader = self.past_master_shards.read();
                    let curr_items = reader.len() * breaking_point as usize;
                    curr_items + local_index as usize
                }
            }
        };

        if data.len() > up_to {
            // Some data didn't fit, insert remaining data into a new shard
            let remaining_data = &data[up_to..];
            let remaining_index = self.raw_insert_rows(remaining_data, true);
            items_pos + remaining_index
        } else {
            items_pos
        }
    }

    fn breaking_point(&self) -> Option<u64> {
        self.current_master_shard.breaking_point()
    }

    pub fn get_element_from_specific(
        &self,
        shard: &S,
        index: usize,
    ) -> Result<Vec<u8>, ShardErrors> {
        shard.read_item_from_index(index)
    }

    pub fn get_element_from_master(&self, index: usize) -> Result<Vec<u8>, ShardErrors> {
        self.get_element_from_specific(&self.current_master_shard, index)
    }

    pub fn get_element(&self, index: usize) -> Result<Vec<u8>, ShardErrors> {
        let breaking_point = self.breaking_point();

        match breaking_point {
            None => self.get_element_from_master(index),
            Some(breaking_point) => {
                let breaking_point_usize = breaking_point as usize;

                let reader = self.past_master_shards.read();
                let shard_reversed = {
                    let mut combined_shards: Vec<&S> = reader.values().rev().collect();
                    combined_shards.push(&self.current_master_shard);
                    combined_shards
                };

                // Calculate the total number of shards
                let num_shards = shard_reversed.len();
                // Determine which shard the index belongs to
                let shard_index = index / breaking_point_usize;

                if shard_index >= num_shards {
                    return Err(ShardErrors::OutOfRange);
                }

                // Calculate the local index within the selected shard
                let local_index = index % breaking_point_usize;

                self.get_element_from_specific(shard_reversed[shard_index], local_index)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fdm::FileDescriptorManager;
    use crate::shard::map_shard::MapShard;
    use crate::shard::shards::data_shard::config::DataShardConfig;
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::Shard;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_context_creation_empty_table() {
        let fake_empty_table_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/fake-db-folder/fake-empty-table");

        let context = MapShard::<DataShard, DataShardConfig>::new(
            fake_empty_table_path,
            "data_",
            DataShardConfig { max_offsets: None },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        assert!(context.past_master_shards.read().is_empty());
        assert_eq!(
            context
                .current_master_shard
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "data_c222a11d-c80f-4d6e-8c8a-7b83f79f9ef2_0.data"
        );
    }

    #[tokio::test]
    pub async fn test_context_creation_partial_table() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/fake-db-folder/fake-partial-folder");
        let context = MapShard::<DataShard, DataShardConfig>::new(
            fake_partial_folder_path,
            "data_",
            DataShardConfig { max_offsets: None },
            Arc::new(FileDescriptorManager::new(2500)),
        );
        assert!(!context.past_master_shards.read().is_empty());
        assert_eq!(
            context
                .current_master_shard
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "data_38af2223-d339-4f45-994e-eef41a69fcaa_2.data"
        );
        assert_eq!(context.past_master_shards.read().len(), 2);
    }

    #[tokio::test]
    pub async fn test_shard_automatic_creation() {
        let fake_partial_folder_path = std::env::current_dir().unwrap().join(format!(
            "./test_cases/fake-db-folder/{}",
            Uuid::new_v4().to_string()
        ));
        std::fs::create_dir(&fake_partial_folder_path).unwrap();

        let context = MapShard::<DataShard, DataShardConfig>::new(
            fake_partial_folder_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(1),
            },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        let map = RwLock::new(context);
        let arc = Arc::new(map);

        let ref_map1 = arc.clone();
        let thread1 = std::thread::spawn(move || {
            ref_map1.write().insert_rows(&[&b"1".to_vec()]);
        });

        let ref_map1 = arc.clone();
        let thread2 = std::thread::spawn(move || {
            ref_map1.write().insert_rows(&[&b"2".to_vec()]);
        });

        thread1.join().unwrap();
        thread2.join().unwrap();

        let map_reader = arc.read();
        let past_reader = map_reader.past_master_shards.read();
        let past_master_shards = past_reader.len();
        assert_eq!(past_master_shards, 1);
        let item1 = past_reader
            .iter()
            .next()
            .unwrap()
            .1
            .read_item_from_index(0)
            .unwrap();
        let item2 = map_reader
            .current_master_shard
            .read_item_from_index(0)
            .unwrap();

        // Collect the items and sort them
        let mut items = vec![item1, item2];
        items.sort();

        assert_eq!(items, vec![b"1".to_vec(), b"2".to_vec()]);

        std::fs::remove_dir_all(fake_partial_folder_path).unwrap();
    }

    #[tokio::test]
    pub async fn test_global_get_element() {
        let fake_partial_folder_path = std::env::current_dir().unwrap().join(format!(
            "./test_cases/fake-db-folder/{}",
            Uuid::new_v4().to_string()
        ));
        std::fs::create_dir(&fake_partial_folder_path).unwrap();

        let mut context = MapShard::<DataShard, DataShardConfig>::new(
            fake_partial_folder_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(1),
            },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        context.insert_rows(&[
            &b"1".to_vec(),
            &b"2".to_vec(),
            &b"3".to_vec(),
            &b"4".to_vec(),
        ]);

        context.get_element(3).unwrap();
    }
}
