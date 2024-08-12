use crate::shard::{Shard, ShardConfig};
use crate::utils::fs::list_files_with_prefix;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct MapShard<S: Shard<Opts>, Opts: ShardConfig> {
    pub current_master_shard: S,
    pub past_master_shards: RwLock<HashMap<String, S>>,
    pub shard_prefix: String,
    pub shards_folder: PathBuf,
    config: Opts,
}

impl<S: Shard<Opts>, Opts: ShardConfig> MapShard<S, Opts> {
    pub fn new<P: AsRef<Path> + Clone>(shards_folder: P, shard_prefix: &str, config: Opts) -> Self {
        let shards_folder = shards_folder.as_ref().to_path_buf();
        let shard_files = list_files_with_prefix(&shards_folder, shard_prefix).unwrap();
        let mut sorted_files: Vec<(usize, String, PathBuf)> = Vec::new();

        for path in shard_files {
            let val = Self::extract_file_signature(path);
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

        let mut past_master_shards = HashMap::new();

        for &(number, ref uuid, ref path) in &sorted_files {
            if path != &current_master_shard {
                past_master_shards.insert(
                    uuid.clone(),
                    S::new(
                        path.clone(),
                        config.clone(),
                        Some(Uuid::parse_str(uuid).unwrap()),
                    ),
                );
            }
        }

        MapShard {
            current_master_shard: S::new(
                current_master_shard,
                config.clone(),
                Some(maybe_new_shard_id),
            ),
            past_master_shards: RwLock::new(past_master_shards),
            shard_prefix: shard_prefix.to_string(),
            shards_folder,
            config,
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

    fn extract_file_signature(path: PathBuf) -> Option<(usize, String, PathBuf)> {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            let parts: Vec<&str> = name_str
                .split('.')
                .next()
                .unwrap_or("")
                .split('_')
                .collect();
            if parts.len() == 3 {
                if let Ok(number) = parts[2].parse::<usize>() {
                    return Some((number, parts[1].to_string(), path));
                }
            }
        }

        None
    }

    pub fn insert_row(&mut self, data: Vec<u8>) {
        let curr_master_has_space = self.current_master_shard.has_space();

        if !curr_master_has_space {
            let (shard_number, _, _) =
                Self::extract_file_signature(self.current_master_shard.get_path().clone()).unwrap();
            let new_shard_number = shard_number + 1;

            let shard = {
                let shard_id = Uuid::new_v4();
                let shard_path = self.shards_folder.clone().join(Self::generate_shard_name(
                    self.shard_prefix.as_str(),
                    shard_id.clone(),
                    new_shard_number,
                ));
                let new_shard = S::new(shard_path, self.config.clone(), Some(shard_id));
                new_shard
            };

            // Add to past master
            {
                let old_master = std::mem::replace(&mut self.current_master_shard, shard);
                let mut past_ms_writer = self.past_master_shards.write().unwrap();
                past_ms_writer.insert(old_master.get_id(), old_master);
            }
        }

        self.current_master_shard.insert_item(data).unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::shard::map_shard::MapShard;
    use crate::shard::shards::data_shard::config::DataShardConfig;
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::Shard;
    use std::sync::{Arc, RwLock};
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
        );

        assert!(context.past_master_shards.read().unwrap().is_empty());
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
        );
        assert!(!context.past_master_shards.read().unwrap().is_empty());
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
        assert_eq!(context.past_master_shards.read().unwrap().len(), 2);
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
        );

        let map = RwLock::new(context);
        let arc = Arc::new(map);

        let ref_map1 = arc.clone();
        let thread1 = std::thread::spawn(move || {
            ref_map1.write().unwrap().insert_row(b"1".to_vec());
        });

        let ref_map1 = arc.clone();
        let thread2 = std::thread::spawn(move || {
            ref_map1.write().unwrap().insert_row(b"2".to_vec());
        });

        thread1.join().unwrap();
        thread2.join().unwrap();

        let map_reader = arc.read().unwrap();
        let past_reader = map_reader.past_master_shards.read().unwrap();
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
}
