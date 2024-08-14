mod query_shard;
mod query_shard_entry;

use crate::managers::single::query_shard::QueryShard;
use crate::primitives::Row;
use schemajs_dirs::create_scheme_js_db;
use std::hash::Hash;
use uuid::Uuid;

pub struct SingleQueryManager<T: Row<T> + Hash> {
    pub shards: Vec<QueryShard<T>>,
    pub num_shards: usize,
    pub scheme: String,
    pub id: Uuid,
}

impl<T: Row<T> + Hash> SingleQueryManager<T> {
    // Initialize the database with empty shards
    pub fn new(scheme: String, num_shards: usize) -> Self {
        let uuid = Uuid::new_v4();

        {
            create_scheme_js_db(
                None,
                format!("{}_{}", scheme.clone(), uuid.to_string()).as_str(),
            );
        }

        let mut shards = Vec::with_capacity(num_shards);
        for _ in 0..num_shards {
            shards.push(QueryShard::new(
                scheme.clone().to_string(),
                uuid.to_string(),
            ));
        }

        SingleQueryManager {
            shards,
            num_shards,
            scheme,
            id: uuid,
        }
    }

    fn get_shard(&self, index: usize) -> &QueryShard<T> {
        &self.shards[index]
    }

    pub fn insert(&self, row: T)
    where
        T: Hash,
    {
        let shard_key = row.get_hashed_shard_key();
        let shard_index = shard_key % self.num_shards as u64;
        let shard = self.get_shard(shard_index as usize);

        {
            shard.insert(row);
        }
    }
}
