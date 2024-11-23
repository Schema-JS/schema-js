use crate::data_handler::DataHandler;
use crate::errors::ShardErrors;
use crate::fdm::FileDescriptorManager;
use crate::shard::insert_item::InsertItem;
use crate::shard::item_type::{ShardItem, ShardItemType};
use crate::shard::map_shard::MapShard;
use crate::shard::shards::data_shard::config::DataShardConfig;
use crate::shard::shards::data_shard::shard_header::DataShardHeader;
use crate::shard::{AvailableSpace, Shard};
use crate::utils::flatten;
use crate::utils::fs::write_at;
use crate::U64_SIZE;
use parking_lot::RwLock;
use std::io::{Error, ErrorKind, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
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
    pub fn read_item(&self, offset_position_in_header: usize) -> Result<ShardItem, ShardErrors> {
        let header_read = self.header.read();

        let item_pos =
            { header_read.get_offset_value_from_offset_header(offset_position_in_header) };

        match item_pos {
            None => Err(ShardErrors::UnknownOffset),
            Some(start_pos) => {
                let data_reader = self.data.read();
                let end_pos = {
                    let next_offset_pos = offset_position_in_header + U64_SIZE;
                    assert!(next_offset_pos <= header_read.max_offset_positions);
                    let mut no_more_positions = false;

                    if header_read.max_offset_positions == next_offset_pos {
                        no_more_positions = true;
                    }

                    let end_reading = if no_more_positions {
                        data_reader.len()
                    } else {
                        header_read
                            .get_offset_value_from_offset_header(next_offset_pos)
                            .map(|e| e as usize)
                            .unwrap_or_else(|| data_reader.len())
                    };

                    end_reading as u64
                };

                let length = (end_pos - start_pos) as usize;

                let read_bytes = data_reader.read_pointer(start_pos, length);
                match read_bytes {
                    None => Err(ShardErrors::ErrorReadingByteRange),
                    Some(b) => Ok({
                        let mut item = ShardItem::from(b);
                        item.set_internal_pos(start_pos as usize);
                        item
                    }),
                }
            }
        }
    }

    pub(crate) fn prepare_item_insert(&self, item: &[u8], item_id: Uuid, offset: usize) -> Vec<u8> {
        ShardItem::new(item, item_id, self.id, offset).to_vec()
    }

    fn find_offsets_by_index(&self, indexes: &[u64]) -> Vec<(u64, u64)> {
        let mut pos_to_modify = vec![];

        for &index in indexes {
            if let Ok(_) = self.read_item_from_index(index as usize) {
                if let Some(off_in_header) =
                    self.header.read().get_offset_pos_by_index(index as usize)
                {
                    if let Some(item_start_pos) = self
                        .header
                        .read()
                        .get_offset_value_from_offset_header(off_in_header)
                    {
                        pos_to_modify.push((item_start_pos, index));
                    }
                }
            }
        }
        pos_to_modify
    }
}

impl Shard<DataShardConfig> for DataShard {
    fn new(
        path: PathBuf,
        opts: DataShardConfig,
        uuid: Option<Uuid>,
        fdm: Arc<FileDescriptorManager>,
    ) -> Self {
        let data_handler = unsafe { DataHandler::new(path.clone(), fdm) }.unwrap();
        let arc_dh = Arc::new(data_handler);
        let header = DataShardHeader::new_from_file(arc_dh.clone(), opts.max_offsets, uuid);

        DataShard {
            path: path.clone(),
            data: arc_dh.clone(),
            id: header.id,
            header: RwLock::new(header),
        }
    }

    fn has_space(&self) -> bool {
        let header = self.header.read();

        header.has_space()
    }

    fn breaking_point(&self) -> Option<u64> {
        Some(self.header.read().get_max_offsets())
    }

    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn get_last_index(&self) -> i64 {
        let header_reader = self.header.read();
        header_reader.get_last_offset_index()
    }

    fn read_item_from_index(&self, index: usize) -> Result<ShardItem, ShardErrors> {
        let header = self.header.read();
        let offset_pos_in_header = header.get_offset_pos_by_index(index);
        match offset_pos_in_header {
            None => Err(ShardErrors::UnknownOffset),
            Some(pos_in_header) => self.read_item(pos_in_header),
        }
    }

    fn available_space(&self) -> AvailableSpace {
        let header = self.header.read();

        AvailableSpace::Fixed(header.available_space())
    }

    fn insert_item(&self, data: &[InsertItem]) -> Result<u64, ShardErrors> {
        let mut header_write = self.header.write();
        let op = self.data.write().operate(|file| {
            // Calculate the current end of the file
            let end_of_file = file
                .seek(SeekFrom::End(0))
                .expect("Failed to seek to end of file");

            let mut offsets = end_of_file;

            let prepare_data: Vec<Vec<u8>> = data
                .iter()
                .map(|row| {
                    let insert =
                        self.prepare_item_insert(row.data, row.uuid.clone(), offsets as usize);
                    offsets += insert.len() as u64;
                    insert
                })
                .collect();

            let write_data: Vec<&[u8]> = prepare_data.iter().map(|row| row.as_slice()).collect();

            let write_data = flatten(write_data);

            // Write the item to the file
            file.write_all(&write_data)
                .expect("Failed to write item to file");

            let mut curr_offset = end_of_file;

            for item in prepare_data {
                header_write
                    .add_next_offset(curr_offset, file)
                    .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Out of position"))?;
                curr_offset += item.len() as u64;
            }

            Ok(header_write.get_last_offset_index())
        });

        match op {
            Ok(offset) => Ok(offset as u64),
            Err(e) => {
                if e.kind() == ErrorKind::OutOfMemory {
                    return Err(ShardErrors::OutOfPositions);
                }
                Err(ShardErrors::FlushingError)
            }
        }
    }

    fn update_items(
        &self,
        data: Vec<(u64, &[u8])>,
        map_shard: &mut MapShard<Self, DataShardConfig>,
    ) -> Result<(), ShardErrors>
    where
        Self: Sized,
    {
        let mut items_to_update = vec![];
        let mut items_to_move = vec![];

        for (index, update_data) in data {
            let item = self.read_item_from_index(index as usize);
            match item {
                Ok(mut res) => {
                    // It already was updated
                    if res.shard_item_type.is_moved() {
                        continue;
                    }

                    res.update(update_data);

                    let internal_pos = res.get_internal_pos().unwrap();

                    if res.shard_item_type.is_moved() {
                        items_to_move.push((internal_pos, update_data.to_vec(), res))
                    } else {
                        items_to_update.push((internal_pos, res));
                    }
                }
                Err(_) => continue,
            }
        }

        self.data
            .write()
            .operate(|file| {
                for (offset, item) in items_to_update {
                    write_at(file, &item.to_vec(), offset as u64)?;
                }

                Ok(())
            })
            .map_err(|_| ShardErrors::FailedUpdate)?;

        self.data
            .write()
            .operate(|file| {
                for (offset, new_item, mut original) in items_to_move {
                    let insert_new_item = map_shard
                        .insert_rows(&[InsertItem::new(&new_item, original.current_item_id)]); // TODO: Insert from data item
                    let shard = {
                        let (shard, local_indx) = map_shard
                            .get_shard_by_global_item_index(insert_new_item)
                            .unwrap();
                        (shard.get_id(), local_indx)
                    };

                    original.moved_shard_id = Some(Uuid::from_str(shard.0.as_str()).unwrap()); // Todo: Unwrap
                    original.moved_offset = Some(shard.1);

                    write_at(file, &original.to_vec(), offset as u64)?;
                }

                Ok(())
            })
            .map_err(|_| ShardErrors::FailedUpdate)?;

        Ok(())
    }

    fn remove_items(&self, indexes: &[u64]) -> Result<(), ShardErrors> {
        let pos_to_modify = self.find_offsets_by_index(indexes);

        let deletion_result = self.data.write().operate(|file| {
            for (pos, _) in pos_to_modify {
                write_at(file, &ShardItemType::Deleted.get_le_identifier(), pos)
                    .map_err(|_| Error::new(ErrorKind::Other, ShardErrors::FailedDeletion))?;
            }
            Ok(())
        });

        if let Err(e) = deletion_result {
            let error = e
                .downcast::<ShardErrors>()
                .unwrap_or_else(|_| ShardErrors::FlushingError);
            return Err(if error.is_failed_deletion() {
                error
            } else {
                ShardErrors::FlushingError
            });
        }

        Ok(())
    }

    fn get_id(&self) -> String {
        self.id.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::errors::ShardErrors;
    use crate::fdm::FileDescriptorManager;
    use crate::shard::insert_item::InsertItem;
    use crate::shard::item_type::{ShardItem, ShardItemType};
    use crate::shard::shards::data_shard::config::DataShardConfig;
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::Shard;
    use std::io::{Error, ErrorKind, Read};
    use std::sync::{Arc, RwLock};
    use std::time::Duration;
    use tempfile::{tempdir, tempfile};
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_err() {
        let a = Error::new(ErrorKind::Other, ShardErrors::FailedDeletion);
        assert!(a.downcast::<ShardErrors>().unwrap().is_failed_deletion());
    }

    #[tokio::test]
    pub async fn test_data_shard() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let config = DataShardConfig {
            max_offsets: Some(10),
        };

        let data_shard = DataShard::new(
            file_path.clone(),
            config,
            None,
            Arc::new(FileDescriptorManager::new(2500)),
        );

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

        let collect_into_slices: Vec<InsertItem> = strs
            .iter()
            .map(|i| InsertItem::new(i.as_bytes(), Uuid::new_v4()))
            .collect();
        data_shard.insert_item(&collect_into_slices).unwrap();

        /*
        unsafe {
            let reader = shards.data.read().unwrap();
            let map = reader.access_map();
            println!("{:?}", map[123..188].to_vec());
            println!("{:?}", reader.read_pointer(123, 65));
        }*/

        let item = data_shard.read_item_from_index(0).unwrap();
        assert_eq!(item.get_used_data(), "Hello World".as_bytes().to_vec());

        let item = data_shard.read_item_from_index(9).unwrap();
        assert_eq!(item.get_used_data(), "String".as_bytes().to_vec());

        let item = data_shard.read_item_from_index(5).unwrap();
        assert_eq!(item.get_used_data(), "1".as_bytes().to_vec());

        let item = data_shard.insert_item(&[InsertItem::new(&vec![1, 2, 3], Uuid::new_v4())]);
        assert!(item.is_err());
        assert!(item.err().unwrap().is_out_of_positions());

        // let res: Vec<u64> = vec![104, 115, 128, 137, 142, 146, 147, 151, 156, 174];
        // assert_eq!(res, shards.header.read().unwrap().offsets);
    }

    #[tokio::test]
    pub async fn test_delete_from_shard() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.bin", Uuid::new_v4().to_string()));

        let config = DataShardConfig {
            max_offsets: Some(4),
        };

        let data_shard = DataShard::new(
            file_path.clone(),
            config,
            None,
            Arc::new(FileDescriptorManager::new(2500)),
        );

        let strs = ["A", "B", "C", "D"];

        let collect_into_slices: Vec<InsertItem> = strs
            .iter()
            .map(|i| InsertItem::new(i.as_bytes(), Uuid::new_v4()))
            .collect();
        data_shard.insert_item(&collect_into_slices).unwrap();

        let item1 = data_shard.read_item_from_index(0).unwrap();
        let item2 = data_shard.read_item_from_index(1).unwrap();
        let item3 = data_shard.read_item_from_index(2).unwrap();
        let item4 = data_shard.read_item_from_index(3).unwrap();

        assert!(item1.shard_item_type.is_raw());
        assert!(item2.shard_item_type.is_raw());
        assert!(item3.shard_item_type.is_raw());
        assert!(item4.shard_item_type.is_raw());

        data_shard.remove_items(&[1, 2]).unwrap();
        let item2 = data_shard.read_item_from_index(1).unwrap();
        let item3 = data_shard.read_item_from_index(2).unwrap();
        assert!(item2.shard_item_type.is_deleted());
        assert!(item3.shard_item_type.is_deleted());
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

        let data_shard = DataShard::new(
            file_path.clone(),
            config,
            None,
            Arc::new(FileDescriptorManager::new(2500)),
        );

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
            data_shard
                .insert_item(&[InsertItem::new(&data.as_bytes().to_vec(), Uuid::new_v4())])
                .unwrap();
        }

        let new_data_shard = DataShard::new(
            file_path.clone(),
            DataShardConfig {
                max_offsets: Some(10),
            },
            None,
            Arc::new(FileDescriptorManager::new(2500)),
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
            Arc::new(FileDescriptorManager::new(2500)),
        );
        let shard = Arc::new(RwLock::new(data_shard));

        let ref_shard = shard.clone();
        let thread_1 = std::thread::spawn(move || {
            ref_shard
                .write()
                .unwrap()
                .insert_item(&[InsertItem::new(b"Hello World", Uuid::new_v4())])
                .unwrap();
        });

        let ref_shard = shard.clone();
        let thread_2 = std::thread::spawn(move || {
            ref_shard
                .write()
                .unwrap()
                .insert_item(&[InsertItem::new(b"Cats are beautiful", Uuid::new_v4())])
                .unwrap();
        });

        thread_1.join().unwrap();
        thread_2.join().unwrap();

        /*let item = shard.read().unwrap().header.read().unwrap().offsets.len();
        assert_eq!(item, 2);*/
    }

    #[tokio::test]
    pub async fn test_data_shard_threads_read_() {
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
            Arc::new(FileDescriptorManager::new(2500)),
        );
        let shard = Arc::new(RwLock::new(data_shard));

        let ref_shard = shard.clone();
        let thread_1 = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            ref_shard
                .write()
                .unwrap()
                .insert_item(&[InsertItem::new(b"Hello World", Uuid::new_v4())])
                .unwrap();
        });
        let a = shard.read().unwrap().read_item_from_index(0);
        assert!(a.is_err());
        tokio::time::sleep(Duration::from_secs(5)).await;
        let a = shard.read().unwrap().read_item_from_index(0);
        assert!(a.is_ok());
        assert_eq!(a.unwrap().get_used_data(), b"Hello World");

        /*let item = shard.read().unwrap().header.read().unwrap().offsets.len();
        assert_eq!(item, 2);*/
    }
}
