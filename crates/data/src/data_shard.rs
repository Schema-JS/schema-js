use crate::data_shard_header::DataShardHeader;
use crate::errors::DataShardErrors;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct DataShard {
    pub file: RwLock<File>,
    pub path: PathBuf,
    pub header: RwLock<DataShardHeader>,
}

impl DataShard {
    pub fn new<P: AsRef<Path> + Clone>(
        path: P,
        max_offsets: Option<u64>,
        uuid: Option<Uuid>,
    ) -> Self {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path.clone())
            .expect("Failed to create shard file");

        let header = DataShardHeader::new_from_file(&mut file, max_offsets, uuid);

        DataShard {
            path: path.as_ref().to_path_buf(),
            file: RwLock::new(file),
            header: RwLock::new(header),
        }
    }

    pub fn read_item_from_index(
        &self,
        header_offset_pos: usize,
    ) -> Result<Vec<u8>, DataShardErrors> {
        let header = self.header.read().unwrap();
        let offset_by_pos = header.offsets.get(header_offset_pos);
        match offset_by_pos {
            None => Err(DataShardErrors::UnknownOffset),
            Some(&offset) => self.read_item(offset),
        }
    }

    /// Reads data of type T from the given position to the next position in offsets
    pub fn read_item(&self, file_offset: u64) -> Result<Vec<u8>, DataShardErrors> {
        let header_read = self.header.read().unwrap();

        // Check if the file_offset exists in the header offsets
        if let Some(pos) = header_read.get_offset_pos(file_offset).ok() {
            let start_offset = header_read.offsets[pos];

            let end_offset = {
                let offsets_len = header_read.offsets.len();
                // Determine the end offset
                let mut end_offset = if offsets_len > pos + 1 {
                    header_read.offsets[pos + 1]
                } else {
                    0
                };

                // Handle case where end_offset might be zero or it indicates reading to the end of the file
                if end_offset == 0 {
                    let metadata = self
                        .file
                        .read()
                        .unwrap()
                        .metadata()
                        .expect("Failed to get file metadata");
                    end_offset = metadata.len();
                }

                end_offset
            };

            let length = (end_offset - start_offset) as usize;
            let mut buffer = vec![0u8; length];

            let mut file = self.file.write().unwrap();
            file.seek(SeekFrom::Start(start_offset))
                .expect("Failed to seek to start_offset");
            file.read_exact(&mut buffer).expect("Failed to read data");

            Ok(buffer)
        } else {
            Err(DataShardErrors::UnknownOffset)
        }
    }

    pub fn insert_item(&self, item: Vec<u8>) -> Result<(), DataShardErrors> {
        let mut header_write = self.header.write().unwrap();
        let mut file = self.file.write().unwrap();

        // Calculate the current end of the file
        let end_of_file = file
            .seek(SeekFrom::End(0))
            .expect("Failed to seek to end of file");

        // Write the item to the file
        file.write_all(&item).expect("Failed to write item to file");

        // Update the header with the new offset
        header_write.add_next_offset(&mut file, end_of_file)?;

        match file.flush() {
            Ok(_) => Ok(()),
            Err(_) => Err(DataShardErrors::FlushingError),
        }
    }

    pub fn has_space(&self) -> bool {
        let header = self.header.read().unwrap();
        // Calculate the number of used offsets
        let used_offsets = header.offsets.iter().filter(|&&offset| offset != 0).count();

        // Check if the used offsets are less than the maximum allowed offsets
        header.get_max_offsets() > used_offsets as u64
    }

    pub fn get_id(&self) -> String {
        self.header.read().unwrap().id.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::data_shard::DataShard;
    use crate::errors::DataShardErrors;
    use std::fs::File;
    use std::sync::{Arc, RwLock};
    use tempfile::{tempdir, tempfile};
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_data_shard() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let data_shard = DataShard::new(file_path, Some(10), None);

        let strs = [
            "Hello World",
            "Cats are cute",
            "Venezuela",
            "Roses",
            "Cars",
            "1",
            "true",
            "false",
            "------Divider-----",
            "String",
        ];

        for data in strs.into_iter() {
            data_shard.insert_item(data.as_bytes().to_vec()).unwrap();
        }

        let item = data_shard.read_item_from_index(9).unwrap();
        assert_eq!(item, "String".as_bytes().to_vec());

        let item = data_shard.read_item_from_index(5).unwrap();
        assert_eq!(item, "1".as_bytes().to_vec());

        let item = data_shard.insert_item(vec![1, 2, 3]);
        assert!(item.is_err());
        assert!(item.err().unwrap().is_out_of_positions());

        let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        assert_eq!(res, data_shard.header.read().unwrap().offsets);
    }

    #[tokio::test]
    pub async fn test_data_shard_from_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));
        let data_shard = DataShard::new(file_path.clone(), Some(10), None);

        let strs = [
            "Hello World",
            "Cats are cute",
            "Venezuela",
            "Roses",
            "Cars",
            "1",
            "true",
            "false",
            "------Divider-----",
            "String",
        ];

        for data in strs.into_iter() {
            data_shard.insert_item(data.as_bytes().to_vec()).unwrap();
        }

        let new_data_shard = DataShard::new(file_path.clone(), Some(10), None);
        let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        assert_eq!(res, new_data_shard.header.read().unwrap().offsets);
    }

    #[tokio::test]
    pub async fn test_data_shard_threads() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let data_shard = DataShard::new(file_path, Some(2), None);
        let shard = Arc::new(RwLock::new(data_shard));

        let ref_shard = shard.clone();
        let thread_1 = std::thread::spawn(move || {
            ref_shard
                .write()
                .unwrap()
                .insert_item(b"Hello World".to_vec())
                .unwrap();
        });

        let ref_shard = shard.clone();
        let thread_2 = std::thread::spawn(move || {
            ref_shard
                .write()
                .unwrap()
                .insert_item(b"Cats are beautiful".to_vec())
                .unwrap();
        });

        thread_1.join().unwrap();
        thread_2.join().unwrap();

        let item = shard.read().unwrap().header.read().unwrap().offsets.len();
        assert_eq!(item, 2);
    }
}
