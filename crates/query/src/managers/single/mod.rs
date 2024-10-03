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
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;
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

    pub data_path: Option<PathBuf>,
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
            data_path: None,
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
                self.data_path.clone(),
                self.scheme.as_str(),
                TempDataShardConfig {
                    max_offsets: TempOffsetTypes::Custom(Some(1000)),
                },
            ),
        );
    }

    pub fn insert_from_value_map(
        &self,
        data: Vec<(String, HashMap<String, DataValue>)>,
        master_insert: bool,
    ) -> Result<Option<Uuid>, QueryError> {
        let mut rows: Vec<T> = data
            .into_iter()
            .map(|e| {
                let table = self
                    .tables
                    .get(&e.0)
                    .ok_or_else(|| QueryError::InvalidTable(e.0.clone()))?
                    .table
                    .clone();
                T::from_map(table, e.1).map_err(|_| QueryError::InvalidInsertion)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.raw_insert(&mut rows, master_insert)
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
    ///   table: query_manager.get_table("users").unwrap(),
    ///   value: RowData {
    ///     value: Default::default()
    ///    }
    /// });
    /// if let Ok(uuid) = uuid {
    /// if let Some(uuid) = uuid {
    ///     println!("Success inserting row. UUID : {}", uuid.to_string());
    /// }
    /// } else {
    ///     panic!("Row could not be inserted")
    /// }
    /// ```
    ///
    /// `SingleQueryManager` will require a folder to be created for `database-name` otherwise it will panic.
    /// For a reference on how this is plugged: crates/query/src/search/search_manager.rs#test_search_manager
    pub fn insert(&self, row: T) -> Result<Option<Uuid>, QueryError> {
        self.raw_insert(&mut [row], false)
    }

    pub fn raw_insert(
        &self,
        rows: &mut [T],
        master_insert: bool,
    ) -> Result<Option<Uuid>, QueryError> {
        let rows_len = rows.len();
        let mut table_inserts: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
        let mut id = None;

        for row in rows.iter_mut() {
            let table_name = row.get_table_name();

            // UUid
            {
                let uuid_col = Table::get_internal_uid();
                let uuid = row.get_value(uuid_col);

                if uuid.is_none() {
                    let uuid = Uuid::new_v4();

                    if rows_len == 1 {
                        id = Some(uuid.clone());
                    }

                    row.set_value(uuid_col, DataValue::Uuid(uuid));
                }
            }

            let serialized_value = row
                .serialize()
                .map_err(|_| QueryError::InvalidSerialization)?;

            table_inserts
                .entry(table_name.clone())
                .or_insert_with(|| Vec::new())
                .push(serialized_value);
        }

        for (table_name, rows) in table_inserts {
            if let Some(table_shard) = self.tables.get(&table_name) {
                let vec_of_slices: Vec<&[u8]> = rows.iter().map(|v| v.as_slice()).collect();
                if !master_insert {
                    table_shard.temps.insert(&vec_of_slices)?;
                } else {
                    let mut data_lock = table_shard
                        .data
                        .write()
                        .map_err(|e| QueryError::InvalidInsertion)?;
                    for row in vec_of_slices {
                        let pointer = data_lock.insert_rows(&[row]);

                        TableShard::<T>::insert_indexes(
                            table_shard.table.clone(),
                            table_shard.indexes.clone(),
                            vec![DataWithIndex {
                                data: row.to_vec(),
                                index: pointer as u64,
                            }],
                        );
                    }
                }
            } else {
                return Err(QueryError::InvalidTable(table_name));
            }
        }

        Ok(id)
    }

    pub fn get_table(&self, table_name: &str) -> Option<Arc<Table>> {
        self.tables.get(table_name).map(|e| e.table.clone())
    }
}
