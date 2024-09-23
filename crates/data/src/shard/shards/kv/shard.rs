use crate::data_handler::DataHandler;
use crate::errors::ShardErrors;
use crate::shard::shards::kv::config::KvShardConfig;
use crate::shard::shards::kv::shard_header::KvShardHeader;
use crate::shard::shards::kv::util::get_element_offset;
use crate::shard::{AvailableSpace, Shard};
use crate::utils::flatten;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct KvShard {
    path: PathBuf,
    pub data: Arc<RwLock<DataHandler>>,
    pub header: RwLock<KvShardHeader>,
    pub value_size: usize,
    max_capacity: usize,
    id: Uuid,
}

impl KvShard {
    pub fn get_element(&self, index: usize) -> Option<Vec<u8>> {
        let reader = self.data.read().unwrap();
        let starting_point = Self::get_element_offset(index, self.value_size) as u64;
        reader.read_pointer(starting_point, self.value_size)
    }

    fn get_element_offset(index: usize, value_size: usize) -> usize {
        get_element_offset(index, value_size)
    }

    pub fn swap_elements(
        &self,
        file: &mut File,
        i: usize,
        first_element: &[u8],
        second_element: &[u8],
    ) -> Result<(), std::io::Error> {
        file.write_at(
            second_element,
            Self::get_element_offset(i, self.value_size) as u64,
        )?;
        file.write_at(
            first_element,
            Self::get_element_offset(i - 1, self.value_size) as u64,
        )?;
        Ok(())
    }
}

impl Shard<KvShardConfig> for KvShard {
    fn new(path: PathBuf, opts: KvShardConfig, uuid: Option<Uuid>) -> Self {
        let data = unsafe { DataHandler::new(path.clone()).unwrap() };
        let data = Arc::new(data);

        let header = KvShardHeader::new_from_file(
            data.clone(),
            uuid,
            Some(0),
            opts.max_capacity,
            opts.value_size as u64,
        );

        Self {
            path,
            data: data.clone(),
            max_capacity: header.max_capacity.unwrap_or(0) as usize,
            value_size: header.value_size as usize,
            id: header.id,
            header: RwLock::new(header),
        }
    }

    fn has_space(&self) -> bool {
        if self.max_capacity == 0 {
            true
        } else {
            self.max_capacity as i64 > self.get_last_index() + 1
        }
    }

    fn breaking_point(&self) -> Option<u64> {
        if self.max_capacity > 0 {
            Some(self.max_capacity as u64)
        } else {
            None
        }
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn get_last_index(&self) -> i64 {
        self.header
            .read()
            .unwrap()
            .items_len
            .checked_sub(1)
            .map_or(-1, |v| v as i64)
    }

    fn read_item_from_index(&self, index: usize) -> Result<Vec<u8>, ShardErrors> {
        match self.get_element(index) {
            None => Err(ShardErrors::UnknownEntry),
            Some(v) => Ok(v),
        }
    }

    fn available_space(&self) -> AvailableSpace {
        if self.max_capacity == 0 {
            AvailableSpace::Unlimited
        } else {
            let space = self.max_capacity - ((self.get_last_index() + 1) as usize);
            AvailableSpace::Fixed(space)
        }
    }

    fn insert_item(&self, data: &[&[u8]]) -> Result<u64, ShardErrors> {
        let mut writer = self.data.write().unwrap();
        writer
            .operate(|file| {
                let _ = file
                    .seek(SeekFrom::End(0))
                    .expect("Failed to seek to end of file");

                let flat_items = flatten(data);

                file.write_all(&flat_items)
                    .expect("Failed to write item to file");

                let new_len = {
                    let new_len = self
                        .header
                        .write()
                        .unwrap()
                        .increment_len(Some(data.len() as u64), file);
                    new_len
                };

                Ok(new_len)
            })
            .map(|e| e)
            .map_err(|e| ShardErrors::ErrorAddingEntry)
    }

    fn get_id(&self) -> String {
        self.id.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::shard::shards::kv::config::KvShardConfig;
    use crate::shard::shards::kv::shard::KvShard;
    use crate::shard::Shard;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_kv_shard() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.index", Uuid::new_v4().to_string()));

        let kv_shard = KvShard::new(
            file_path,
            KvShardConfig {
                value_size: 1,
                max_capacity: None,
            },
            None,
        );

        kv_shard
            .insert_item(&[
                &"a".to_string().into_bytes(),
                &"b".to_string().into_bytes(),
                &"c".to_string().into_bytes(),
            ])
            .unwrap();

        assert_eq!(kv_shard.header.read().unwrap().items_len, 3);

        assert_eq!(
            kv_shard.get_element(1).unwrap(),
            "b".to_string().into_bytes()
        );
        assert_eq!(
            kv_shard.get_element(2).unwrap(),
            "c".to_string().into_bytes()
        );
        assert_eq!(
            kv_shard.get_element(0).unwrap(),
            "a".to_string().into_bytes()
        );

        assert!(kv_shard.get_element(3).is_none(),);
    }
}
