mod table_shard;

use crate::errors::QueryError;
use crate::managers::single::table_shard::TableShard;
use crate::ops::query_ops::QueryOps;
use crate::primitives::Row;
use crate::utils::index_utils::{matching_indexes, matching_indexes_for_query};
use chashmap::{CHashMap, ReadGuard};
use schemajs_data::shard::shards::data_shard::config::TempDataShardConfig;
use schemajs_data::temp_offset_types::TempOffsetTypes;
use schemajs_primitives::table::Table;
use std::hash::Hash;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct SingleQueryManager<T: Row<T>> {
    pub tables: Arc<CHashMap<String, TableShard<T>>>,
    pub num_shards: usize,
    pub scheme: String,
    pub id: Uuid,
}

impl<T: Row<T>> SingleQueryManager<T> {
    // Initialize the database with empty shards
    pub fn new(scheme: String, num_shards: usize) -> Self {
        let uuid = Uuid::new_v4();

        SingleQueryManager {
            num_shards,
            tables: Arc::new(CHashMap::default()),
            scheme,
            id: uuid,
        }
    }

    pub fn search(&self, table: String, query: QueryOps) {
        let table = self.tables.get(&table);
        match table {
            None => {}
            Some(tbl) => {
                let potential_indx = matching_indexes_for_query(&tbl.table.indexes, &query);
                if !potential_indx.is_empty() {
                    let index = potential_indx
                        .iter()
                        .min_by_key(|i| i.members.len())
                        .unwrap();
                }
            }
        }
    }

    pub fn register_table(&self, table: Table) {
        self.tables.insert(
            table.name.clone(),
            TableShard::<T>::new(
                table,
                None,
                self.scheme.as_str(),
                TempDataShardConfig {
                    max_offsets: TempOffsetTypes::Custom(Some(1000)),
                },
            ),
        );
    }

    pub fn insert(&self, row: T) -> Result<Uuid, QueryError> {
        let table_name = row.get_table_name();
        let table = self.tables.get(&table_name);

        if let Some(table_shard) = table {
            let uuid = row
                .get_value(&Table::get_internal_uid())
                .ok_or(QueryError::UnknownUid)?;

            let serialized_value = row
                .serialize()
                .map_err(|e| QueryError::InvalidSerialization)?;

            table_shard.temps.insert(serialized_value)?;

            Ok(uuid.as_uuid().unwrap().clone())
        } else {
            Err(QueryError::InvalidTable(table_name))
        }
    }
}
