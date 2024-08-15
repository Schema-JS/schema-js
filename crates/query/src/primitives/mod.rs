use crate::serializer::RowSerializer;
use ahash::AHasher;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use std::hash::{Hash, Hasher};

/// Trait for providing the sharding key
pub trait ShardKey {
    fn hash_key(&self, key: String) -> u64 {
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

pub trait Row<T>: RowSerializer<T> + ShardKey + From<Vec<u8>> {
    fn get_value(&self, column: &Column) -> Option<DataValue>;
    fn get_table_name(&self) -> String;
    fn validate(&self) -> bool;
}
