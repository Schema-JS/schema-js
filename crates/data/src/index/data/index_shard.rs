use crate::errors::ShardErrors;
use crate::index::data::index_data_unit::IndexDataUnit;
use crate::index::types::{IndexKey, IndexValue};
use crate::index::utils::get_entry_size;
use crate::shard::map_shard::MapShard;
use crate::shard::shards::kv::config::KvShardConfig;
use crate::shard::shards::kv::shard::KvShard;
use crate::shard::Shard;
use crate::U64_SIZE;
use std::cmp::Ordering;
use std::io::{Seek, Write};
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug)]
pub struct IndexShard<K: IndexKey, V: IndexValue> {
    pub data: RwLock<MapShard<KvShard, KvShardConfig>>,
    binary_order: bool,
    key_size: usize,
    value_size: usize,

    // Markers
    _key_marker: PhantomData<K>,
    _val_marker: PhantomData<V>,
}

pub type IndexEntry = (IndexDataUnit, IndexDataUnit, Vec<u8>);

impl<K: IndexKey, V: IndexValue> IndexShard<K, V> {
    pub fn new<P: AsRef<Path> + Clone>(
        shard_folder: P,
        index_name: String,
        key_size: usize,
        value_size: usize,
        max_capacity: Option<u64>,
        binary_order: Option<bool>,
    ) -> Self {
        let shard_collection = MapShard::new(
            shard_folder.as_ref().to_path_buf(),
            format!("indx{}_", index_name).as_str(),
            KvShardConfig {
                value_size: get_entry_size(key_size, value_size),
                max_capacity: max_capacity.clone(),
            },
        );

        Self {
            data: RwLock::new(shard_collection),
            binary_order: binary_order.unwrap_or(false),
            _key_marker: PhantomData,
            _val_marker: PhantomData,
            key_size,
            value_size,
        }
    }

    pub fn build_entry_from_vec(&self, el: Vec<u8>) -> Option<IndexEntry> {
        let index_unit = IndexDataUnit::try_from(el.as_slice()).ok()?;
        let data = index_unit.data;
        let key = IndexDataUnit::try_from(&data[0..(U64_SIZE + self.key_size)]).ok()?;
        let value = IndexDataUnit::try_from(&data[(U64_SIZE + self.key_size)..]).ok()?;

        Some((key, value, el))
    }

    pub fn get_entry_from_shard(
        &self,
        shard: &KvShard,
        index: usize,
    ) -> Result<Vec<u8>, ShardErrors> {
        shard.read_item_from_index(index)
    }

    pub fn get_entry(&self, index: usize, global: bool) -> Option<IndexEntry> {
        let get_el = if !global {
            self.data.read().unwrap().get_element_from_master(index)
        } else {
            self.data.read().unwrap().get_element(index)
        };

        match get_el {
            Ok(el) => Some(self.build_entry_from_vec(el)?),
            Err(_) => None,
        }
    }

    pub fn get_kv(&self, index: usize, global: bool) -> Option<(K, V, Vec<u8>)> {
        let entry = self.get_entry(index, global);
        match entry {
            None => return None,
            Some((key_unit, val_unit, el)) => Some(self.build_kv(key_unit, val_unit, el)),
        }
    }

    pub fn build_kv(
        &self,
        key_unit: IndexDataUnit,
        val_unit: IndexDataUnit,
        el: Vec<u8>,
    ) -> (K, V, Vec<u8>) {
        (K::from(key_unit), V::from(val_unit), el)
    }

    pub fn insert(&self, key: K, value: V) {
        let key_vec: Vec<u8> = key.into();
        let value_vec: Vec<u8> = value.into();

        let build_entry = self.build_entry(key_vec, value_vec);
        let entry_index_unit: Vec<u8> = build_entry.into();

        self.data.write().unwrap().insert_row(entry_index_unit);

        if self.binary_order {
            self.keep_binary_order();
        }
    }

    pub fn binary_search(&self, target: K) -> Option<(u64, K, V)> {
        let reader = self.data.read().unwrap();
        let breaking_point = reader.current_master_shard.breaking_point();
        match breaking_point {
            None => self.raw_binary_search(&reader.current_master_shard, target),
            Some(_) => {
                let past_master_shards = reader.past_master_shards.read().unwrap();

                let shards = {
                    let mut shards = vec![&reader.current_master_shard];
                    let combined_shards: Vec<&KvShard> = past_master_shards.values().collect();
                    shards.extend(combined_shards);
                    shards
                };

                for shard in shards {
                    if let Some(found) = self.raw_binary_search(shard, target.clone()) {
                        return Some(found);
                    }
                }

                None
            }
        }
    }

    pub fn raw_binary_search(&self, shard: &KvShard, target: K) -> Option<(u64, K, V)> {
        let mut left = 0;
        let mut right = shard.get_last_index();

        while left <= right {
            let mid = left + (right - left) / 2;

            let kv = {
                let entry = self.get_entry_from_shard(shard, mid as usize).unwrap();
                let (key_unit, val_unit, el) = self.build_entry_from_vec(entry).unwrap();
                self.build_kv(key_unit, val_unit, el)
            };

            let (key, value, _) = kv;

            match key.cmp(&target) {
                Ordering::Less => {
                    left = mid + 1;
                }
                Ordering::Equal => {
                    return Some((mid as u64, key, value));
                }
                _ => {
                    right = mid.saturating_sub(1);
                }
            }
        }

        None
    }

    fn build_entry(&self, key: Vec<u8>, value: Vec<u8>) -> IndexDataUnit {
        let build_entry = {
            let mut entry: Vec<u8> = Vec::new();

            let key = IndexDataUnit::new(key);
            let value = IndexDataUnit::new(value);

            let key_vec_val: Vec<u8> = key.into();
            let value_vec_val: Vec<u8> = value.into();

            entry.extend(key_vec_val);
            entry.extend(value_vec_val);
            entry
        };

        IndexDataUnit::new(build_entry)
    }

    fn keep_binary_order(&self) {
        let mut i = {
            self.data
                .read()
                .unwrap()
                .current_master_shard
                .get_last_index()
        };

        while i > 0 {
            let (curr_index, _, curr_original_el) = self.get_kv(i as usize, false).unwrap();
            let (prev_index, _, prev_original_el) = self.get_kv(i as usize - 1, false).unwrap();

            match curr_index.cmp(&prev_index) {
                Ordering::Less => {
                    let mut writer = self.data.write().unwrap();
                    let mut curr_shard = writer.current_master_shard.data.write().unwrap();
                    curr_shard
                        .operate(|file| {
                            writer
                                .current_master_shard
                                .swap_elements(
                                    file,
                                    i as usize,
                                    &curr_original_el,
                                    &prev_original_el,
                                )
                                .unwrap();
                            i -= 1;
                            Ok(())
                        })
                        .unwrap();
                }
                _ => break,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::index::data::index_shard::IndexShard;
    use crate::index::keys::string_index::StringIndexKey;
    use crate::index::utils::get_entry_size;
    use crate::index::vals::raw_value::RawIndexValue;
    use crate::U64_SIZE;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_data_positions() {
        let entry_size = get_entry_size(32, 1024);
        assert_eq!(entry_size, 1080);
        //
        // let offset_by_index = get_element_offset(1, 32, 1024);
        // assert_eq!(
        //     offset_by_index,
        //     IndexShardHeader::header_size() + (entry_size * 1)
        // );
    }

    #[tokio::test]
    pub async fn test_inserts_and_gets() {
        let temp_dir = tempdir().unwrap();
        let index_folder = temp_dir.path().join("indx");

        std::fs::create_dir(index_folder.clone()).unwrap();

        let mut index = IndexShard::new(
            index_folder.clone(),
            "indx".to_string(),
            32,
            1024,
            None,
            Some(true),
        );

        let key_size = 32;
        let value_size = 1024;

        index.insert(
            StringIndexKey("a".repeat(key_size)),
            RawIndexValue(vec![0u8; 1024]),
        );
        index.insert(StringIndexKey("b".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("c".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("d".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("e".repeat(key_size)), vec![1u8; 1024].into());
        index.insert(StringIndexKey("f".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("g".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("h".repeat(key_size)), vec![0u8; 1024].into());

        {
            let entry = index.get_kv(0, true).unwrap();
            assert_eq!(entry.0, StringIndexKey("a".repeat(key_size)));
            assert_eq!(entry.1 .0, vec![0u8; 1024]);

            let entry = index.get_kv(4, true).unwrap();
            assert_eq!(entry.0, StringIndexKey("e".repeat(key_size)));
            assert_eq!(entry.1 .0, [1u8; 1024]);

            let entry = index.get_kv(7, true).unwrap();
            assert_eq!(entry.0, StringIndexKey("h".repeat(key_size)));
            assert_eq!(entry.1 .0, [0u8; 1024]);

            let entry = index.get_kv(8, true);
            assert!(entry.is_none())
        }

        std::fs::remove_dir_all(index_folder).unwrap();
    }

    #[tokio::test]
    pub async fn test_binary_order() {
        let temp_dir = tempdir().unwrap();
        let index_folder = temp_dir.path().join("indx");

        std::fs::create_dir(index_folder.clone()).unwrap();

        let mut index = IndexShard::new(
            index_folder.clone(),
            "indx".to_string(),
            32,
            1024,
            None,
            Some(true),
        );

        let key_size = 32;
        let value_size = 1024;

        index.insert(
            StringIndexKey("z".repeat(key_size)),
            RawIndexValue(vec![0u8; 1024]),
        );
        index.insert(StringIndexKey("h".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("i".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("j".repeat(key_size)), vec![1u8; 1024].into());
        index.insert(StringIndexKey("b".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("d".repeat(key_size)), vec![0u8; 1024].into());
        index.insert(StringIndexKey("e".repeat(key_size)), vec![0u8; 1024].into());

        assert_eq!(index.get_kv(0, true).unwrap().0 .0, "b".repeat(key_size));
        assert_eq!(index.get_kv(1, true).unwrap().0 .0, "d".repeat(key_size));
        assert_eq!(index.get_kv(2, true).unwrap().0 .0, "e".repeat(key_size));
        assert_eq!(index.get_kv(3, true).unwrap().0 .0, "h".repeat(key_size));
        assert_eq!(index.get_kv(4, true).unwrap().0 .0, "i".repeat(key_size));
        assert_eq!(index.get_kv(5, true).unwrap().0 .0, "j".repeat(key_size));
        assert_eq!(index.get_kv(6, true).unwrap().0 .0, "z".repeat(key_size));

        std::fs::remove_dir_all(index_folder).unwrap();
    }

    #[tokio::test]
    pub async fn test_binary_order_with_fixed_size_keys() {
        let temp_dir = tempdir().unwrap();
        let index_folder = temp_dir.path().join("indx");

        std::fs::create_dir(index_folder.clone()).unwrap();

        let mut index = IndexShard::new(
            index_folder.clone(),
            "indx".to_string(),
            32,
            1024,
            None,
            Some(true),
        );

        let pad_key = |s: &str| -> String {
            let mut key = s.to_string();
            key.truncate(32); // Ensure the key is no longer than 32 characters
            while key.len() < 32 {
                key.push(' '); // Pad with spaces to make it 32 characters long
            }
            key
        };

        // Insert keys with custom format, padded to 32 characters
        index.insert(
            StringIndexKey(pad_key("string(a:2)")),
            RawIndexValue(vec![0u8; 1024]),
        );
        index.insert(
            StringIndexKey(pad_key("string(a:0)")),
            vec![1u8; 1024].into(),
        );
        index.insert(
            StringIndexKey(pad_key("string(a:1)")),
            vec![2u8; 1024].into(),
        );

        // Assert the correct order
        assert_eq!(index.get_kv(0, true).unwrap().0 .0, pad_key("string(a:0)"));
        assert_eq!(index.get_kv(1, true).unwrap().0 .0, pad_key("string(a:1)"));
        assert_eq!(index.get_kv(2, true).unwrap().0 .0, pad_key("string(a:2)"));

        // Check that the values correspond correctly
        assert_eq!(index.get_kv(0, true).unwrap().1 .0, vec![1u8; 1024]);
        assert_eq!(index.get_kv(1, true).unwrap().1 .0, vec![2u8; 1024]);
        assert_eq!(index.get_kv(2, true).unwrap().1 .0, vec![0u8; 1024]);

        std::fs::remove_dir_all(index_folder).unwrap();
    }
}
