use crate::index::data::index_shard::IndexShard;
use crate::index::implementations::hash::hash_index_header::{
    HASH_INDEX_KEY_SIZE, HASH_INDEX_TOTAL_ENTRY_SIZE, HASH_INDEX_VALUE_SIZE,
};
use crate::index::keys::index_key_sha256::IndexKeySha256;
use crate::index::vals::raw_value::RawIndexValue;
use crate::index::Index;
use std::fmt::Debug;
use std::io::{Seek, Write};
use std::path::Path;

#[derive(Debug)]
pub struct HashIndex {
    index: IndexShard<IndexKeySha256, RawIndexValue>,
}

impl HashIndex {
    fn new_from_path<P: AsRef<Path> + Clone>(path: P) -> Self {
        let index_shard = IndexShard::new(path, Some(true));

        Self { index: index_shard }
    }

    fn calculate_item_offset(&self, index: usize) -> usize {
        HASH_INDEX_TOTAL_ENTRY_SIZE * index
    }

    pub fn find_index(&self, find: IndexKeySha256) -> Option<u64> {
        match self
            .index
            .binary_search(find, HASH_INDEX_KEY_SIZE, HASH_INDEX_VALUE_SIZE)
        {
            None => return None,
            Some((_, _, val)) => Some(u64::from_le_bytes(val.0.as_slice().try_into().unwrap())),
        }
    }
}

impl Index<IndexKeySha256> for HashIndex {
    fn insert(&mut self, key: IndexKeySha256, row_position: u64) {
        self.index
            .insert(key, row_position.to_le_bytes().to_vec().into());
    }

    fn get(&self, key: &IndexKeySha256) -> Option<u64> {
        todo!()
    }

    fn remove(&mut self, key: &IndexKeySha256) -> Option<u64> {
        todo!()
    }

    fn search(&self, key: &IndexKeySha256) -> Option<u64> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crate::index::composite_key::composite_key::CompositeKey;
    use crate::index::implementations::hash::hash_index::HashIndex;
    use crate::index::keys::index_key_sha256::IndexKeySha256;
    use crate::index::Index;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_binary_search_with_composite_keys() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join(format!("{}.index", Uuid::new_v4().to_string()));

        let mut index = HashIndex::new_from_path(file_path);

        let usernames = vec![
            String::from("user1"),
            String::from("user2"),
            String::from("user3"),
            String::from("user4"),
            String::from("user5"),
            String::from("user6"),
            String::from("user7"),
            String::from("user8"),
            String::from("user9"),
            String::from("user10"),
            String::from("user11"),
            String::from("user12"),
            String::from("user13"),
            String::from("user14"),
            String::from("user15"),
            String::from("user16"),
            String::from("user17"),
            String::from("user18"),
            String::from("user19"),
            String::from("user20"),
            String::from("user21"),
            String::from("user22"),
            String::from("user23"),
            String::from("user24"),
            String::from("user25"),
        ];

        let cities = vec![
            String::from("City1"),
            String::from("City2"),
            String::from("City3"),
            String::from("City4"),
            String::from("City5"),
            String::from("City6"),
            String::from("City7"),
            String::from("City8"),
            String::from("City9"),
            String::from("City10"),
            String::from("City11"),
            String::from("City12"),
            String::from("City13"),
            String::from("City14"),
            String::from("City15"),
            String::from("City16"),
            String::from("City17"),
            String::from("City18"),
            String::from("City19"),
            String::from("City20"),
            String::from("City21"),
            String::from("City22"),
            String::from("City23"),
            String::from("City24"),
            String::from("City25"),
        ];

        for i in 0..25 {
            let composite_key = CompositeKey(vec![
                (String::from("username"), usernames[i].clone()),
                (String::from("city"), cities[i].clone()),
            ]);

            let key = IndexKeySha256::from(composite_key);
            println!("Adding {}", key.to_string());

            index.insert(key, rand::random());
        }

        println!("After loop");

        let key = CompositeKey(vec![
            (String::from("username"), String::from("user16")),
            (String::from("city"), String::from("City16")),
        ]);

        let key: IndexKeySha256 = key.into();
        let val = index.find_index(key.clone());

        println!("Searching for key: {}", key.to_string());

        assert!(val.is_some());

        let val = index.find_index(
            CompositeKey(vec![
                (String::from("username"), String::from("user25")),
                (String::from("city"), String::from("City25")),
            ])
            .into(),
        );
        assert!(val.is_some());

        let val = index.find_index(
            CompositeKey(vec![
                (String::from("username"), String::from("user14")),
                (String::from("city"), String::from("City16")),
            ])
            .into(),
        );
        assert!(val.is_none());

        // Search All
        for i in 0..25 {
            let composite_key = CompositeKey(vec![
                (String::from("username"), usernames[i].clone()),
                (String::from("city"), cities[i].clone()),
            ]);

            let key = IndexKeySha256::from(composite_key);
            assert!(index.find_index(key).is_some());
        }
    }
}
