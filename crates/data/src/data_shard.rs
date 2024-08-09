use crate::data_handler::DataHandler;
use crate::data_shard_header::DataShardHeader;
use crate::errors::DataShardErrors;
use crate::U64_SIZE;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct DataShard {
    pub path: PathBuf,
    pub header: RwLock<DataShardHeader>,
    pub data: Arc<RwLock<DataHandler>>,
}

impl DataShard {
    pub fn new<P: AsRef<Path> + Clone>(
        path: P,
        max_offsets: Option<u64>,
        uuid: Option<Uuid>,
    ) -> Self {
        let data_handler = unsafe { DataHandler::new(path.clone()) }.unwrap();
        let arc_dh = Arc::new(data_handler);

        let header = DataShardHeader::new_from_file(arc_dh.clone(), max_offsets, uuid);

        DataShard {
            path: path.as_ref().to_path_buf(),
            data: arc_dh.clone(),
            header: RwLock::new(header),
        }
    }

    pub fn read_item_from_index(&self, index: usize) -> Result<Vec<u8>, DataShardErrors> {
        let header = self.header.read().unwrap();
        let offset_pos_in_header = header.get_offset_pos_by_index(index);
        match offset_pos_in_header {
            None => Err(DataShardErrors::UnknownOffset),
            Some(pos_in_header) => self.read_item(pos_in_header),
        }
    }

    /// Reads data of type T from the given position to the next position in offsets
    pub fn read_item(&self, offset_position_in_header: usize) -> Result<Vec<u8>, DataShardErrors> {
        let header_read = self.header.read().unwrap();

        let item_pos =
            { header_read.get_offset_value_from_offset_header(offset_position_in_header) };

        match item_pos {
            None => Err(DataShardErrors::UnknownOffset),
            Some(start_pos) => {
                let data_reader = self.data.read().unwrap();
                let end_pos = {
                    let next_offset_pos = offset_position_in_header + U64_SIZE;
                    assert!(next_offset_pos <= header_read.max_offset_positions);
                    let mut no_more_positions = false;

                    if header_read.max_offset_positions == next_offset_pos {
                        no_more_positions = true;
                    }

                    let end_reading = {
                        if no_more_positions {
                            Some(0)
                        } else {
                            header_read.get_offset_value_from_offset_header(next_offset_pos)
                        }
                    };

                    // Item might have not been inserted yet, so we read till the end of the file
                    let read_up_to = if let Some(end_pos) = end_reading {
                        if end_pos == 0 {
                            data_reader.len() as u64
                        } else {
                            end_pos
                        }
                    } else {
                        return Err(DataShardErrors::UnknownOffset);
                    };

                    read_up_to
                };

                let length = (end_pos - start_pos) as usize;

                let read_bytes = data_reader.read_pointer(start_pos, length);
                match read_bytes {
                    None => Err(DataShardErrors::ErrorReadingByteRange),
                    Some(b) => Ok(b),
                }
            }
        }
    }

    pub fn insert_item(&self, item: Vec<u8>) -> Result<(), DataShardErrors> {
        let has_space = { self.has_space() };
        if has_space {
            let mut header_write = self.header.write().unwrap();
            let op = self.data.write().unwrap().operate(|file| {
                // Calculate the current end of the file
                let end_of_file = file
                    .seek(SeekFrom::End(0))
                    .expect("Failed to seek to end of file");

                // Write the item to the file
                file.write_all(&item).expect("Failed to write item to file");

                // Update the header with the new offset
                header_write.add_next_offset(end_of_file, file).unwrap();

                Ok(())
            });

            match op {
                Ok(_) => Ok(()),
                Err(_) => Err(DataShardErrors::FlushingError),
            }
        } else {
            Err(DataShardErrors::OutOfPositions)
        }
    }

    pub fn has_space(&self) -> bool {
        let header = self.header.read().unwrap();

        header.has_space()
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
    use std::io::Read;
    use std::sync::{Arc, RwLock};
    use tempfile::{tempdir, tempfile};
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_data_shard() {
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
        /*
        unsafe {
            let reader = data_shard.data.read().unwrap();
            let map = reader.access_map();
            println!("{:?}", map[123..188].to_vec());
            println!("{:?}", reader.read_pointer(123, 65));
        }*/

        let item = data_shard.read_item_from_index(0).unwrap();
        assert_eq!(item, "Hello World".as_bytes().to_vec());

        let item = data_shard.read_item_from_index(9).unwrap();
        assert_eq!(item, "String".as_bytes().to_vec());

        let item = data_shard.read_item_from_index(5).unwrap();
        assert_eq!(item, "1".as_bytes().to_vec());

        let item = data_shard.insert_item(vec![1, 2, 3]);
        assert!(item.is_err());
        assert!(item.err().unwrap().is_out_of_positions());

        // let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        // assert_eq!(res, data_shard.header.read().unwrap().offsets);
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
        /*let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        assert_eq!(res, new_data_shard.header.read().unwrap().offsets);*/
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

        /*let item = shard.read().unwrap().header.read().unwrap().offsets.len();
        assert_eq!(item, 2);*/
    }
}
