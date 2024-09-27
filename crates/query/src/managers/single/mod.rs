pub mod table_shard;

use crate::errors::QueryError;
use crate::managers::single::table_shard::TableShard;
use crate::row::Row;
use crate::search::search_manager::QuerySearchManager;
use chashmap::CHashMap;
use schemajs_data::shard::shards::data_shard::config::TempDataShardConfig;
use schemajs_data::shard::temp_map_shard::DataWithIndex;
use schemajs_data::temp_offset_types::TempOffsetTypes;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct SingleQueryManager<T: Row<T>> {
    // A thread-safe vector that holds the names of registered tables.
    // This is used to track the tables managed by the query manager.
    pub table_names: RwLock<Vec<String>>,

    // A reference-counted concurrent hash map that stores the registered tables and their associated TableShard instances.
    // Each table is mapped to its corresponding TableShard, which handles data management and sharding.
    pub tables: Arc<CHashMap<String, TableShard<T>>>,

    // The schema or organization structure of the database, stored as a string.
    // This is used when interacting with tables and managing their sharding structure.
    pub scheme: String,

    // A unique identifier for this instance of SingleQueryManager.
    // This UUID helps in distinguishing different query managers in the system.
    pub id: Uuid,

    pub search_manager: QuerySearchManager<T>,
}

/// `SingleQueryManager` is responsible for managing all query-related operations
/// on a collection of tables in a database. It provides methods for registering tables,
/// handling data insertions, and managing table shards. This struct is designed to work
/// with a generic row type `T` that implements the `Row` trait, allowing it to be flexible
/// in managing different types of rows across tables.
///
/// Each table is associated with a `TableShard`, which handles the sharding of data and
/// the management of temporary shards for efficient data insertions. The `SingleQueryManager`
/// also maintains thread-safe access to a list of table names and table shards through
/// `RwLock` and `Arc<CHashMap>`, ensuring it can be used in concurrent environments.
///
/// Fields:
/// - `table_names`: A list of registered table names, maintained in a thread-safe manner.
/// - `tables`: A thread-safe hash map of table names to their corresponding `TableShard` instances.
/// - `scheme`: The schema or structure of the database, as a string.
/// - `id`: A unique identifier for the query manager instance, useful for distinguishing between
///          multiple query managers in a larger system.
impl<T: Row<T>> SingleQueryManager<T> {
    /// Initializes an instance of SingleQueryManager which handles everything query-related
    ///
    /// Examples
    ///
    /// ```
    /// use schemajs_query::managers::single::SingleQueryManager;
    /// use schemajs_query::row_json::RowJson;
    /// let query_manager: SingleQueryManager<RowJson> = SingleQueryManager::new("database-name".to_string());
    /// ```
    pub fn new(scheme: String) -> Self {
        let uuid = Uuid::new_v4();
        let tables = Arc::new(CHashMap::default());

        SingleQueryManager {
            table_names: RwLock::new(vec![]),
            tables: tables.clone(),
            scheme,
            id: uuid,
            search_manager: QuerySearchManager::new(tables),
        }
    }

    /// Register a table and creates a shard manager for insertions (`TableShard`)
    /// This method already handles the initialization of: Main map shard, Temp shards, and indexes.
    /// When creating a table it ideally must be created following `Table::new(name: &str)`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use schemajs_primitives::table::Table;
    /// use schemajs_query::managers::single::SingleQueryManager;
    /// use schemajs_query::row_json::RowJson;
    ///
    /// let query_manager: SingleQueryManager<RowJson> = SingleQueryManager::new("database-name".to_string());
    /// query_manager.register_table(Table::new("users"));
    /// ```
    ///
    /// Note `register_table` will panic due to `No such file or directory` due to the database must have a folder already created in system.
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

    /// Inserts a row in the first available temporary shard.
    /// This method will intentionally reconcile to the master shard IF and only IF the temporary shard runs out of spots.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use schemajs_primitives::table::Table;
    /// use schemajs_query::managers::single::SingleQueryManager;
    /// use schemajs_query::row::Row;
    /// use schemajs_query::row_json::{RowData, RowJson};
    ///
    ///
    /// let query_manager = SingleQueryManager::new("database-name".to_string());
    /// query_manager.register_table(Table::new("users"));
    ///
    /// let uuid = query_manager.insert(RowJson {
    ///   value: RowData {
    ///     table: "users".to_string(),
    ///     value: Default::default()
    ///    }
    /// });
    /// if let Ok(uuid) = uuid {
    ///     println!("Success inserting row. UUID : {}", uuid.to_string());
    /// } else {
    ///     panic!("Row could not be inserted")
    /// }
    /// ```
    ///
    /// `SingleQueryManager` will require a folder to be created for `database-name` otherwise it will panic.
    /// For a reference on how this is plugged: crates/query/src/search/search_manager.rs#test_search_manager
    pub fn insert(&self, row: T) -> Result<Uuid, QueryError> {
        self.raw_insert(row, false)
    }

    pub fn raw_insert(&self, mut row: T, master_insert: bool) -> Result<Uuid, QueryError> {
        let table_name = row.get_table_name();
        let table = self.tables.get(&table_name);

        if let Some(table_shard) = table {
            let uuid_col = Table::get_internal_uid();
            let uuid = row
                .get_value(&uuid_col)
                .unwrap_or_else(|| DataValue::Uuid(Uuid::new_v4()));

            row.set_value(&uuid_col, uuid.clone());

            let serialized_value = row
                .serialize()
                .map_err(|e| QueryError::InvalidSerialization)?;

            if !master_insert {
                table_shard.temps.insert(&serialized_value)?;
            } else {
                let pointer = table_shard
                    .data
                    .write()
                    .unwrap()
                    .insert_rows(&[&serialized_value]);

                TableShard::<T>::insert_indexes(
                    table_shard.table.clone(),
                    table_shard.indexes.clone(),
                    vec![DataWithIndex {
                        data: serialized_value,
                        index: pointer as u64,
                    }],
                );
            }

            Ok(uuid.as_uuid().unwrap().clone())
        } else {
            Err(QueryError::InvalidTable(table_name))
        }
    }
}
