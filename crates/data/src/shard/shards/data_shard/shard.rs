use crate::data_handler::DataHandler;
use crate::errors::ShardErrors;
use crate::shard::map_shard::MapShard;
use crate::shard::shards::data_shard::config::DataShardConfig;
use crate::shard::shards::data_shard::shard_header::DataShardHeader;
use crate::shard::Shard;
use crate::U64_SIZE;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct DataShard {
    pub path: PathBuf,
    pub header: RwLock<DataShardHeader>,
    pub data: Arc<RwLock<DataHandler>>,
    pub id: Uuid,
}

impl DataShard {
    /// Reads data of type T from the given position to the next position in offsets
    pub fn read_item(&self, offset_position_in_header: usize) -> Result<Vec<u8>, ShardErrors> {
        let header_read = self.header.read().unwrap();

        let item_pos =
            { header_read.get_offset_value_from_offset_header(offset_position_in_header) };

        match item_pos {
            None => Err(ShardErrors::UnknownOffset),
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
                        return Err(ShardErrors::UnknownOffset);
                    };

                    read_up_to
                };

                let length = (end_pos - start_pos) as usize;

                let read_bytes = data_reader.read_pointer(start_pos, length);
                match read_bytes {
                    None => Err(ShardErrors::ErrorReadingByteRange),
                    Some(b) => Ok(b),
                }
            }
        }
    }
}

impl Shard<DataShardConfig> for DataShard {
    fn new(path: PathBuf, opts: DataShardConfig, uuid: Option<Uuid>) -> Self {
        let data_handler = unsafe { DataHandler::new(path.clone()) }.unwrap();
        let arc_dh = Arc::new(data_handler);
        let header = DataShardHeader::new_from_file(arc_dh.clone(), opts.max_offsets, uuid);

        DataShard {
            path: path.clone(),
            data: arc_dh.clone(),
            id: header.id,
            header: RwLock::new(header),
        }
    }

    fn get_last_index(&self) -> i64 {
        let header_reader = self.header.read().unwrap();
        let last_index = header_reader.get_last_offset_index();

        last_index
    }

    fn read_item_from_index(&self, index: usize) -> Result<Vec<u8>, ShardErrors> {
        let header = self.header.read().unwrap();
        let offset_pos_in_header = header.get_offset_pos_by_index(index);
        match offset_pos_in_header {
            None => Err(ShardErrors::UnknownOffset),
            Some(pos_in_header) => self.read_item(pos_in_header),
        }
    }

    fn breaking_point(&self) -> Option<u64> {
        Some(self.header.read().unwrap().get_max_offsets())
    }

    fn has_space(&self) -> bool {
        let header = self.header.read().unwrap();

        header.has_space()
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn insert_item(&self, data: Vec<u8>) -> Result<(), ShardErrors> {
        let has_space = { self.has_space() };
        if has_space {
            let mut header_write = self.header.write().unwrap();
            let op = self.data.write().unwrap().operate(|file| {
                // Calculate the current end of the file
                let end_of_file = file
                    .seek(SeekFrom::End(0))
                    .expect("Failed to seek to end of file");

                // Write the item to the file
                file.write_all(&data).expect("Failed to write item to file");

                // Update the header with the new offset
                header_write.add_next_offset(end_of_file, file).unwrap();

                Ok(())
            });

            match op {
                Ok(_) => Ok(()),
                Err(_) => Err(ShardErrors::FlushingError),
            }
        } else {
            Err(ShardErrors::OutOfPositions)
        }
    }

    fn get_id(&self) -> String {
        self.id.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::errors::ShardErrors;
    use crate::shard::shards::data_shard::config::DataShardConfig;
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::Shard;
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

        let config = DataShardConfig {
            max_offsets: Some(10),
        };

        let data_shard = DataShard::new(file_path.clone(), config, None);

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
            let reader = shards.data.read().unwrap();
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
        // assert_eq!(res, shards.header.read().unwrap().offsets);
    }

    #[tokio::test]
    pub async fn test_data_shard_from_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let config = DataShardConfig {
            max_offsets: Some(10),
        };

        let data_shard = DataShard::new(file_path.clone(), config, None);

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

        let new_data_shard = DataShard::new(
            file_path.clone(),
            DataShardConfig {
                max_offsets: Some(10),
            },
            None,
        );
        /*let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        assert_eq!(res, new_data_shard.header.read().unwrap().offsets);*/
    }

    #[tokio::test]
    pub async fn test_data_shard_threads() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let data_shard = DataShard::new(
            file_path,
            DataShardConfig {
                max_offsets: Some(2),
            },
            None,
        );
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
