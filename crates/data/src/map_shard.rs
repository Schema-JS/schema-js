use crate::data_shard::DataShard;
use crate::utils::fs::list_files_with_prefix;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct MapShard {
    pub current_master_shard: RwLock<DataShard>,
    pub past_master_shards: RwLock<HashMap<String, DataShard>>,
}

impl MapShard {
    pub fn new<P: AsRef<Path> + Clone>(shards_folder: P, shard_prefix: &str) -> Self {
        let shards_folder = shards_folder.as_ref().to_path_buf();
        let shard_files = list_files_with_prefix(&shards_folder, shard_prefix).unwrap();
        let mut sorted_files: Vec<(usize, String, PathBuf)> = Vec::new();

        for path in shard_files {
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
                        sorted_files.push((number, parts[1].to_string(), path));
                    }
                }
            }
        }

        sorted_files.sort_by_key(|&(number, _, _)| number);

        let maybe_new_shard_id = Uuid::new_v4();
        let current_master_shard = sorted_files
            .last()
            .map(|&(_, _, ref path)| path.clone())
            .unwrap_or_else(|| {
                shards_folder.join(format!(
                    "{}{}_0.data",
                    shard_prefix,
                    maybe_new_shard_id.to_string()
                ))
            });

        let mut past_master_shards = HashMap::new();

        for &(number, ref uuid, ref path) in &sorted_files {
            if path != &current_master_shard {
                past_master_shards.insert(
                    uuid.clone(),
                    DataShard::new(path.clone(), None, Some(Uuid::parse_str(uuid).unwrap())),
                );
            }
        }

        MapShard {
            current_master_shard: RwLock::new(DataShard::new(
                current_master_shard,
                None,
                Some(maybe_new_shard_id),
            )),
            past_master_shards: RwLock::new(past_master_shards),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::map_shard::MapShard;

    #[tokio::test]
    pub async fn test_context_creation_empty_table() {
        let fake_empty_table_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/fake-db-folder/fake-empty-table");
        let context = MapShard::new(fake_empty_table_path, "data_");
        assert!(context.past_master_shards.read().unwrap().is_empty());
        assert_eq!(
            context
                .current_master_shard
                .read()
                .unwrap()
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
        let context = MapShard::new(fake_partial_folder_path, "data_");
        assert!(!context.past_master_shards.read().unwrap().is_empty());
        assert_eq!(
            context
                .current_master_shard
                .read()
                .unwrap()
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "data_38af2223-d339-4f45-994e-eef41a69fcaa_2.data"
        );
        assert_eq!(context.past_master_shards.read().unwrap().len(), 2);
    }
}
