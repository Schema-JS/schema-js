use crate::data_handler::DataHandler;
use crate::index::data::index_data_unit::IndexDataUnit;
use crate::index::data::index_shard_header::IndexShardHeader;
use crate::index::types::IndexKey;
use crate::U64_SIZE;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct IndexShard {
    pub data: Arc<RwLock<DataHandler>>,
    pub header: RwLock<IndexShardHeader>,
    binary_order: bool,
}

impl IndexShard {
    pub fn new<P: AsRef<Path> + Clone>(shard_file: P, binary_order: Option<bool>) -> Self {
        let data = unsafe { DataHandler::new(shard_file).unwrap() };
        let data = Arc::new(data);
        Self {
            data: data.clone(),
            header: RwLock::new(IndexShardHeader::new_from_file(
                data.clone(),
                Some(0),
                Some(2_500_000),
            )),
            binary_order: binary_order.unwrap_or(false),
        }
    }

    pub fn get_entry_size(key_size: usize, value_size: usize) -> usize {
        let entry_data_size = {
            // The data of an entry is made of 2 IndexDataUnit (Key, Value)
            // + The value of the key and the size.
            let key_size = IndexDataUnit::header_size() + key_size;
            let value_size = IndexDataUnit::header_size() + value_size;
            key_size + value_size
        };
        IndexDataUnit::header_size() + entry_data_size
    }

    pub fn get_element_offset(index: usize, key_size: usize, value_size: usize) -> usize {
        let index_header_size = IndexShardHeader::header_size();
        index_header_size + (index * (Self::get_entry_size(key_size, value_size)))
    }

    pub fn get_element(&self, index: usize, key_size: usize, value_size: usize) -> Option<Vec<u8>> {
        let reader = self.data.read().unwrap();
        let starting_point = Self::get_element_offset(index, key_size, value_size) as u64;
        reader.read_pointer(starting_point, Self::get_entry_size(key_size, value_size))
    }

    pub fn get_entry(
        &self,
        index: usize,
        key_size: usize,
        value_size: usize,
    ) -> Option<(IndexDataUnit, IndexDataUnit)> {
        let get_el = self.get_element(index, key_size, value_size);

        match get_el {
            None => return None,
            Some(el) => {
                let index_unit = IndexDataUnit::from_vec(el).unwrap();
                let data = index_unit.data;
                let key = {
                    let key_vec = (&data[0..(U64_SIZE + key_size)]).to_vec();
                    IndexDataUnit::from_vec(key_vec).unwrap()
                };

                let value_vec = &data[(U64_SIZE + key_size)..];
                let value = IndexDataUnit::from_vec(value_vec.to_vec()).unwrap();

                Some((key, value))
            }
        }
    }

    fn build_entry(&self, key: Vec<u8>, value: Vec<u8>) -> IndexDataUnit {
        let build_entry = {
            let mut entry = Vec::new();
            let key = IndexDataUnit::new(key.clone().into());
            let value = IndexDataUnit::new(value.clone());
            entry.extend(key.to_vec());
            entry.extend(value.to_vec());
            entry
        };

        IndexDataUnit::new(build_entry)
    }

    fn swap_elements<K: IndexKey>(
        &self,
        file: &mut File,
        i: usize,
        key_size: usize,
        value_size: usize,
        first_element: &[u8],
        second_element: &[u8],
    ) -> Result<(), std::io::Error> {
        file.write_at(
            second_element,
            Self::get_element_offset(i, key_size, value_size) as u64,
        )?;
        file.write_at(
            first_element,
            Self::get_element_offset(i - 1, key_size, value_size) as u64,
        )?;
        Ok(())
    }

    fn keep_binary_order<K: IndexKey>(&mut self, key_size: usize, value_size: usize) {
        let mut i = { self.header.read().unwrap().items_len - 1 };

        while i > 0 {
            let (current_key, _) = self.get_entry(i as usize, key_size, value_size).unwrap();
            let (previous_key, _) = self
                .get_entry(i as usize - 1, key_size, value_size)
                .unwrap();

            let curr_index: K = K::from(current_key);
            let prev_index: K = K::from(previous_key);
            match curr_index.cmp(&prev_index) {
                Ordering::Less => {
                    let first_element = self.get_element(i as usize, key_size, value_size).unwrap();
                    let second_element = self
                        .get_element(i as usize - 1, key_size, value_size)
                        .unwrap();

                    {
                        let mut writer = self.data.write().unwrap();
                        writer
                            .operate(|file| {
                                self.swap_elements::<K>(
                                    file,
                                    i as usize,
                                    key_size,
                                    value_size,
                                    &first_element,
                                    &second_element,
                                )
                                .unwrap();
                                i -= 1;
                                Ok(())
                            })
                            .unwrap();
                    }
                }
                _ => break,
            }
        }
    }

    pub fn insert<K: IndexKey>(&mut self, key: K, value: Vec<u8>) {
        let key_vec: Vec<u8> = key.into();
        let key_size = key_vec.len();
        let value_size = value.len();

        {
            let mut writer = self.data.write().unwrap();
            writer
                .operate(|file| {
                    let end_of_file = file
                        .seek(SeekFrom::End(0))
                        .expect("Failed to seek to end of file");

                    let build_entry = self.build_entry(key_vec, value);
                    let entry_index_unit = build_entry.to_vec();

                    file.write_all(&entry_index_unit)
                        .expect("Failed to write item to file");
                    let new_len = {
                        let new_len = self.header.write().unwrap().increment_len(file);
                        new_len
                    };

                    Ok(new_len)
                })
                .unwrap();
        }

        if self.binary_order {
            self.keep_binary_order::<K>(key_size, value_size)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::index::data::index_data_unit::IndexDataUnit;
    use crate::index::data::index_shard::IndexShard;
    use crate::index::data::index_shard_header::IndexShardHeader;
    use crate::index::keys::string_index::StringIndexKey;
    use crate::U64_SIZE;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_data_positions() {
        let entry_size = IndexShard::get_entry_size(32, 1024);
        assert_eq!(entry_size, 1080);

        let offset_by_index = IndexShard::get_element_offset(1, 32, 1024);
        assert_eq!(
            offset_by_index,
            IndexShardHeader::header_size() + (entry_size * 1)
        );
    }

    #[tokio::test]
    pub async fn test_inserts_and_gets() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.index", Uuid::new_v4().to_string()));

        let mut index = IndexShard::new(file_path, Some(true));

        let key_size = 32;
        let value_size = 1024;

        index.insert(StringIndexKey("a".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("b".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("c".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("d".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("e".repeat(key_size)), vec![1u8; 1024]);
        index.insert(StringIndexKey("f".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("g".repeat(key_size)), vec![0u8; 1024]);
        index.insert(StringIndexKey("h".repeat(key_size)), vec![0u8; 1024]);

        {
            let get_el = index.get_element(4, key_size, value_size).unwrap();
            println!("{:?}", &get_el);
            let index_unit = IndexDataUnit::from_vec(get_el).unwrap();
            let data = index_unit.data;
            println!("Data {:?}", data);

            let key_vec = (&data[0..(U64_SIZE + key_size)]).to_vec();
            println!("key vec {:?}", &key_vec);
            let key = IndexDataUnit::from_vec(key_vec).unwrap();
            assert_eq!(key.item_size, key_size as u64);
            assert_eq!("e".repeat(32).into_bytes(), key.data);

            let value_vec = &data[(U64_SIZE + key_size)..];
            let value = IndexDataUnit::from_vec(value_vec.to_vec()).unwrap();
            assert_eq!(value.data, vec![1u8; 1024]);
        }

        {
            let get_el = index.get_element(7, key_size, value_size).unwrap();
            println!("{:?}", &get_el);
            let index_unit = IndexDataUnit::from_vec(get_el).unwrap();
            let data = index_unit.data;
            println!("Data {:?}", data);

            let key_vec = (&data[0..(U64_SIZE + key_size)]).to_vec();
            println!("key vec {:?}", &key_vec);
            let key = IndexDataUnit::from_vec(key_vec).unwrap();
            assert_eq!(key.item_size, key_size as u64);
            assert_eq!("h".repeat(32).into_bytes(), key.data);

            let value_vec = &data[(U64_SIZE + key_size)..];
            let value = IndexDataUnit::from_vec(value_vec.to_vec()).unwrap();
            assert_eq!(value.data, vec![0u8; 1024]);

            let b_entry = index.get_entry(1, key_size, value_size).unwrap();

            assert_eq!(b_entry.0.data, "b".repeat(32).into_bytes());
            assert_eq!(b_entry.1.data, vec![0u8; 1024]);
        }
    }
}
