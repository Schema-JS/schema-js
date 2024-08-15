pub mod query_shard;
pub mod query_shard_entry;

use crate::managers::single::query_shard::QueryShard;
use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_dirs::create_scheme_js_db;
use schemajs_primitives::table::Table;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct SingleQueryManager<T: Row<T>> {
    pub shards: Vec<QueryShard<T>>,
    pub tables: Arc<CHashMap<String, Table>>,
    pub num_shards: usize,
    pub scheme: String,
    pub id: Uuid,
}

impl<T: Row<T>> SingleQueryManager<T> {
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
            tables: Arc::new(CHashMap::default()),
            scheme,
            id: uuid,
        }
    }

    fn get_shard(&self, index: usize) -> &QueryShard<T> {
        &self.shards[index]
    }

    pub fn insert(&self, row: T) -> Option<Uuid> {
        let table_name = row.get_table_name();
        let table = self.tables.get(&table_name);

        if let Some(table) = table {
            let primary_key = table.primary_key.clone();
            let column = table.get_column(primary_key.as_str()).unwrap(); // TODO
            let val = row.get_value(column).unwrap(); // TODO
            let shard_key = row.hash_key(val.to_string());

            let shard_index = shard_key % self.num_shards as u64;
            let shard = self.get_shard(shard_index as usize);

            Some(shard.insert(table.clone(), row))
        } else {
            None
        }
    }
}
