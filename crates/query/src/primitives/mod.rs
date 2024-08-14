use crate::serializer::RowSerializer;
use ahash::AHasher;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::hash::{Hash, Hasher};

/// Trait for providing the sharding key
pub trait ShardKey<T: Hash> {
    fn get_shard_key(&self) -> T;

    fn get_hashed_shard_key(&self) -> u64 {
        let key = self.get_shard_key();
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

pub trait Row<T: Hash>: RowSerializer<T> + ShardKey<T> + From<Vec<u8>> {
    fn get_value(&self, column: String) -> Option<DataValue>;
    fn get_table(&self) -> &Table;
    fn get_table_name(&self) -> String;
    fn validate(&self) -> bool;
}
