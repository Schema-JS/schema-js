use crate::composite_key::CompositeKey;
use crate::data::index_shard::IndexShard;
use crate::implementations::hash::hash_index_header::{
    HASH_INDEX_KEY_SIZE, HASH_INDEX_TOTAL_ENTRY_SIZE, HASH_INDEX_VALUE_SIZE,
};
use crate::index_keys::IndexKeyType;
use crate::keys::index_key_sha256::IndexKeySha256;
use crate::types::Index;
use crate::vals::raw_value::RawIndexValue;
use std::fmt::Debug;
use std::io::{Seek, Write};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct HashIndex {
    pub index: Arc<IndexShard<IndexKeySha256, RawIndexValue>>,
}

impl HashIndex {
    pub fn new_from_path<P: AsRef<Path> + Clone>(
        path: P,
        index_name: Option<String>,
        capacity: Option<u64>,
    ) -> Self {
        let index_shard = IndexShard::new(
            path,
            index_name.unwrap_or_else(|| "hashindx".to_string()),
            HASH_INDEX_KEY_SIZE,
            HASH_INDEX_VALUE_SIZE,
            capacity,
            Some(true),
        );

        Self {
            index: Arc::new(index_shard),
        }
    }

    fn calculate_item_offset(&self, index: usize) -> usize {
        HASH_INDEX_TOTAL_ENTRY_SIZE * index
    }

    pub fn find_index(&self, find: IndexKeySha256) -> Option<u64> {
        match self.index.binary_search(find) {
            None => return None,
            Some((_, _, val)) => Some(u64::from_le_bytes(val.0.as_slice().try_into().unwrap())),
        }
    }
}

impl Index for HashIndex {
    fn to_key(&self, key: CompositeKey) -> IndexKeyType {
        IndexKeyType::Sha256(IndexKeySha256::from(key))
    }

    fn bulk_insert(&self, data: Vec<(IndexKeyType, u64)>) {
        self.index.raw_insert(
            data.into_iter()
                .map(|i| {
                    (
                        i.0.into_sha256().unwrap(),
                        i.1.to_le_bytes().to_vec().into(),
                    )
                })
                .collect(),
        )
    }

    fn insert(&self, key: IndexKeyType, row_position: u64) {
        let key = key.into_sha256().unwrap();
        self.index
            .insert(key, row_position.to_le_bytes().to_vec().into());
    }

    fn get(&self, key: &IndexKeyType) -> Option<u64> {
        self.find_index(key.clone().into_sha256().unwrap())
    }

    fn remove(&mut self, key: &IndexKeyType) -> Option<u64> {
        todo!()
    }

    fn supported_search_operators(&self) -> Vec<String> {
        vec![String::from("=")]
    }
}

#[cfg(test)]
mod test {
    use crate::composite_key::CompositeKey;
    use crate::implementations::hash::hash_index::HashIndex;
    use crate::index_keys::IndexKeyType;
    use crate::keys::index_key_sha256::IndexKeySha256;
    use crate::types::Index;
    use schemajs_data::fdm::FileDescriptorManager;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_binary_search_with_composite_keys() {
        FileDescriptorManager::init(2500);
        let temp_dir = tempdir().unwrap();

        let hashindx = temp_dir.as_ref().to_path_buf().join("hashindx");
        std::fs::create_dir(hashindx.clone()).unwrap();

        let mut index = HashIndex::new_from_path(hashindx.clone(), None, None);

        add_data(&mut index);

        std::fs::remove_dir_all(hashindx).unwrap();
    }

    #[tokio::test]
    pub async fn test_binary_search_with_composite_keys_and_limit() {
        FileDescriptorManager::init(2500);
        let temp_dir = tempdir().unwrap();

        let hashindx = temp_dir.as_ref().to_path_buf().join("hashindx");
        std::fs::create_dir(hashindx.clone()).unwrap();

        // This will create a shard every two elements
        let mut index = HashIndex::new_from_path(hashindx.clone(), None, Some(2));

        add_data(&mut index);

        std::fs::remove_dir_all(hashindx).unwrap();
    }

    fn add_data(index: &mut HashIndex) {
        FileDescriptorManager::init(2500);
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

            index.insert(IndexKeyType::Sha256(key), rand::random());
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
