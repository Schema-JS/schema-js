use crate::row::Row;
use chashmap::CHashMap;
use schemajs_data::shard::map_shard::MapShard;
use schemajs_data::shard::shards::data_shard::config::{DataShardConfig, TempDataShardConfig};
use schemajs_data::shard::shards::data_shard::shard::DataShard;
use schemajs_data::shard::temp_collection::TempCollection;
use schemajs_data::shard::temp_map_shard::DataWithIndex;
use schemajs_dirs::create_schema_js_table;
use schemajs_index::composite_key::CompositeKey;
use schemajs_index::implementations::hash::hash_index::HashIndex;
use schemajs_index::index_keys::IndexKeyType;
use schemajs_index::index_type::{IndexType, IndexTypeValue};
use schemajs_index::types::{Index, IndexKey};
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// `TableShard` is a structure that manages the sharding of a specific table's data.
/// It is responsible for storing the table's data in a main shard, handling temporary shards
/// for efficient insertion, and managing the indexes associated with the table.
///
/// The generic type `T` represents the type of rows the table holds, and it must implement the `Row` trait.
///
/// # Fields:
/// - `table`: The `Table` structure representing the schema of the table being managed.
/// - `data`: An `Arc<RwLock<MapShard<DataShard, DataShardConfig>>>` that represents the main shard where the table's data is stored.
///   This is a thread-safe reference to the shard, which allows concurrent reads and writes to the data.
/// - `temps`: A `TempCollection<DataShard, DataShardConfig, TempDataShardConfig>` that manages temporary shards for storing data
///   before it is reconciled into the main shard. Temporary shards allow for faster writes and efficient sharding operations.
/// - `indexes`: An `Arc<CHashMap<String, IndexTypeValue>>` that contains the table's indexes, stored in a thread-safe concurrent hash map.
///   The key is the index name, and the value is an `IndexTypeValue`, which holds the actual index structure.
///
/// - `_marker`: A `PhantomData<T>` used to indicate the generic type `T` in the struct.
///   It is a marker used to tell the Rust compiler that this struct works with a specific row type,
///   even though it doesn’t directly store a `T`.
#[derive(Debug)]
pub struct TableShard<T: Row> {
    pub table: Arc<Table>,
    pub data: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
    pub temps: TempCollection<DataShard, DataShardConfig, TempDataShardConfig>,
    pub indexes: Arc<CHashMap<String, IndexTypeValue>>,
    _marker: PhantomData<T>,
}

impl<T: Row> TableShard<T> {
    /// Creates a new `TableShard` instance for a given table. This method is responsible for setting up
    /// the table's main data shard, temporary shards, and indexes.
    ///
    /// # Parameters:
    /// - `table`: The `Table` object representing the structure of the table to be sharded.
    /// - `base_path`: An optional base path for the table files. If not provided, a default path will be used.
    /// - `scheme`: The database schema that organizes how the table's data and indexes are structured.
    /// - `temp_config`: Configuration for the temporary shard that handles data before being reconciled with the main shard.
    ///
    /// # Returns:
    /// - A `TableShard` instance that handles data storage, sharding, and indexing for the provided table.
    pub fn new(
        table: Table,
        base_path: Option<PathBuf>,
        scheme: &str,
        temp_config: TempDataShardConfig,
    ) -> Self {
        let table_path = create_schema_js_table(base_path, scheme, table.name.as_str());
        println!("{:?}", table_path);

        let map_shard = MapShard::new(
            table_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(2_500_000),
            },
        );

        let refs = Arc::new(RwLock::new(map_shard));

        let temps_folder = table_path.join("temps");

        if !temps_folder.exists() {
            std::fs::create_dir_all(temps_folder.clone()).unwrap();
        }

        let temp_collection =
            TempCollection::new(refs.clone(), 5, temps_folder, "temp_", temp_config);

        let mut indexes = CHashMap::new();

        for index in &table.indexes {
            let path = table_path.join("indx");

            if !path.exists() {
                std::fs::create_dir(path.clone()).unwrap();
            }

            let index_obj = match index.index_type {
                IndexType::Hash => IndexTypeValue::Hash(HashIndex::new_from_path(
                    path,
                    Some(format!("{}", index.name)),
                    Some(10_000_000),
                )),
            };

            indexes.insert(index.name.clone(), index_obj);
        }

        let mut tbl_shard = Self {
            indexes: Arc::new(indexes),
            data: refs.clone(),
            table: Arc::new(table),
            temps: temp_collection,
            _marker: PhantomData,
        };

        tbl_shard.init();

        tbl_shard
    }

    /// Initializes everything related to the current table context.
    /// Such as loading the indexes
    /// Setting the reconciliation callbacks
    /// and potentially future logic related to table loading.
    pub fn init(&mut self) {
        let indexes = self.indexes.clone();

        for temp_shard in self.temps.temps.iter() {
            let indexes = indexes.clone();
            let table = self.table.clone();

            temp_shard
                .write()
                .unwrap()
                .set_on_reconcile(Box::new(move |rows| {
                    Self::insert_indexes(table.clone(), indexes.clone(), rows);
                    Ok(())
                }))
        }
    }

    /// This method handles automatically indexing the rows that match the index in the Table.
    /// It is called during the reconciling process through `set_on_reconcile` in the TempMapShard.
    pub fn insert_indexes(
        table: Arc<Table>,
        indexes: Arc<CHashMap<String, IndexTypeValue>>,
        data: Vec<DataWithIndex>,
    ) {
        let mut index_ordered_items: HashMap<String, Vec<(IndexKeyType, u64)>> = HashMap::new();

        for row in data {
            let row_t = T::from_slice(&row.data, table.clone());
            for index in &table.indexes {
                let mut can_index = false;
                let mut composite_key_vals: Vec<(String, String)> = vec![];

                for index_col in &index.members {
                    let val = row_t
                        .get_value(table.get_column(index_col).unwrap())
                        .unwrap_or(DataValue::Null);

                    if !val.is_null() {
                        can_index = true;
                    }

                    composite_key_vals.push((index_col.clone(), val.to_string()))
                }

                if can_index {
                    let real_indx = indexes.get(&index.name).unwrap();
                    let composite_key = CompositeKey(composite_key_vals);
                    let indx = real_indx.as_index();
                    let key = indx.to_key(composite_key);
                    let insertion_value = (key, row.index);

                    let index_exists = index_ordered_items.contains_key(&index.name);

                    if index_exists {
                        index_ordered_items
                            .get_mut(&index.name)
                            .unwrap()
                            .push(insertion_value);
                    } else {
                        index_ordered_items.insert(index.name.clone(), vec![insertion_value]);
                    }
                }
            }
        }

        for (index, rows) in index_ordered_items {
            let index = indexes.get_mut(&index).unwrap();
            let indx = index.as_index();

            indx.bulk_insert(rows);
        }
    }
}
