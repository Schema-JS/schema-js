use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_data::shard::map_shard::MapShard;
use schemajs_data::shard::shards::data_shard::config::{DataShardConfig, TempDataShardConfig};
use schemajs_data::shard::shards::data_shard::shard::DataShard;
use schemajs_data::shard::temp_collection::TempCollection;
use schemajs_dirs::create_schema_js_table;
use schemajs_index::composite_key::CompositeKey;
use schemajs_index::implementations::hash::hash_index::HashIndex;
use schemajs_index::index_type::{IndexType, IndexTypeValue};
use schemajs_index::types::{Index, IndexKey};
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct TableShard<T: Row<T>> {
    pub table: Table,
    pub data: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
    pub temps: TempCollection<DataShard, DataShardConfig, TempDataShardConfig>,
    pub indexes: Arc<CHashMap<String, IndexTypeValue>>,
    _marker: PhantomData<T>,
}

impl<T: Row<T>> TableShard<T> {
    pub fn new(
        table: Table,
        base_path: Option<PathBuf>,
        scheme: &str,
        temp_config: TempDataShardConfig,
    ) -> Self {
        let table_path = create_schema_js_table(base_path, scheme, table.name.as_str());

        let map_shard = MapShard::new(
            table_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(2_500_000),
            },
        );

        let refs = Arc::new(RwLock::new(map_shard));

        let temp_collection = TempCollection::new(
            refs.clone(),
            5,
            table_path.join("temps"),
            "temp_",
            temp_config,
        );

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
            table,
            temps: temp_collection,
            _marker: PhantomData,
        };

        tbl_shard.init();

        tbl_shard
    }

    pub fn init(&mut self) {
        let indexes = self.indexes.clone();
        let table = self.table.clone();

        for temp_shard in self.temps.temps.iter() {
            let indexes = indexes.clone();
            let table = table.clone();

            temp_shard
                .write()
                .unwrap()
                .set_on_reconcile(Box::new(move |row, pos| {
                    let row: T = T::from(row.clone());
                    Self::insert_indexes(table.clone(), indexes.clone(), &row, pos);
                    Ok(())
                }))
        }
    }

    pub fn insert_indexes(
        table: Table,
        indexes: Arc<CHashMap<String, IndexTypeValue>>,
        data: &T,
        pos_index: usize,
    ) {
        for index in &table.indexes {
            let mut can_index = false;
            let mut composite_key_vals: Vec<(String, String)> = vec![];

            // Loop over each column in the index
            for index_col in &index.members {
                let val = data
                    .get_value(table.get_column(index_col).unwrap())
                    .unwrap_or(DataValue::Null);

                if !val.is_null() {
                    can_index = true;
                }

                composite_key_vals.push((index_col.clone(), val.to_string()))
            }

            if can_index {
                let index = indexes.get_mut(&(index.name.clone())).unwrap();

                let composite_key = CompositeKey(composite_key_vals);
                let indx = index.as_index();
                let key = indx.to_key(composite_key);

                indx.insert(key, pos_index as u64)
            }
        }
    }
}
