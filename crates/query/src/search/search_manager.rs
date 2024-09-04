use crate::errors::QueryError;
use crate::managers::single::table_shard::TableShard;
use crate::ops::query_ops::QueryOps;
use crate::row::Row;
use crate::utils::index_utils::create_query_plan;
use chashmap::CHashMap;
use std::sync::Arc;

pub struct QuerySearchManager<T: Row<T>> {
    table_shards: Arc<CHashMap<String, TableShard<T>>>,
}

impl<T: Row<T>> QuerySearchManager<T> {
    pub fn new(table_shards: Arc<CHashMap<String, TableShard<T>>>) -> Self {
        Self { table_shards }
    }

    pub fn search(&self, table_name: String, ops: QueryOps) -> Result<Vec<T>, QueryError> {
        let get_table_shard = self
            .table_shards
            .get(&table_name)
            .ok_or_else(|| QueryError::InvalidTable(table_name.clone()))?;
        let potential_indexes = &get_table_shard.table.indexes;
        let find_potential_index = create_query_plan(&ops, potential_indexes);

        Ok(vec![])
    }
}
