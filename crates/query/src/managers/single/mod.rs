pub mod table_shard;

use crate::errors::QueryError;
use crate::managers::single::table_shard::TableShard;
use crate::row::Row;
use chashmap::CHashMap;
use schemajs_data::shard::shards::data_shard::config::TempDataShardConfig;
use schemajs_data::temp_offset_types::TempOffsetTypes;
use schemajs_primitives::table::Table;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct SingleQueryManager<T: Row<T>> {
    pub table_names: RwLock<Vec<String>>,
    pub tables: Arc<CHashMap<String, TableShard<T>>>,
    pub scheme: String,
    pub id: Uuid,
}

impl<T: Row<T>> SingleQueryManager<T> {
    // Initialize the database with empty shards
    pub fn new(scheme: String) -> Self {
        let uuid = Uuid::new_v4();

        SingleQueryManager {
            table_names: RwLock::new(vec![]),
            tables: Arc::new(CHashMap::default()),
            scheme,
            id: uuid,
        }
    }

    pub fn register_table(&self, table: Table) {
        self.table_names.write().unwrap().push(table.name.clone());
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

        // TODO: Config to generate an UUID if not present

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
