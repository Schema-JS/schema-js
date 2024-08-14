use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_data::index::composite_key::CompositeKey;
use schemajs_data::index::implementations::hash::hash_index::HashIndex;
use schemajs_data::index::keys::index_key_sha256::IndexKeySha256;
use schemajs_data::index::Index;
use schemajs_data::shard::shard_collection::ShardCollection;
use schemajs_data::shard::shards::data_shard::config::{DataShardConfig, TempDataShardConfig};
use schemajs_data::shard::shards::data_shard::shard::DataShard;
use schemajs_data::temp_offset_types::TempOffsetTypes;
use schemajs_dirs::create_schema_js_table;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct QueryShardEntry<T: Row<T> + Hash> {
    pub data: ShardCollection<DataShard, DataShardConfig, TempDataShardConfig>,
    pub table: Table,
    pub indexes: Arc<CHashMap<String, HashIndex>>,
    pub path: PathBuf,
    pub uuid: Uuid,
    // Markers
    _key_marker: PhantomData<T>,
}

impl<T: Row<T> + Hash> QueryShardEntry<T> {
    pub fn new(scheme_name: String, table_name: String, table: Table) -> Self {
        let uuid = Uuid::new_v4();
        let table_path = create_schema_js_table(
            None,
            scheme_name.as_str(),
            format!("{}_{}", table_name, uuid.to_string()).as_str(),
        );

        let mut shard_col = ShardCollection::new(
            table_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(2_500_000),
            },
            TempDataShardConfig {
                max_offsets: TempOffsetTypes::Custom(Some(1000)),
            },
        );

        let mut indexes = CHashMap::new();

        for index in &table.indexes {
            indexes.insert(
                index.name.clone(),
                HashIndex::new_from_path(
                    table_path.join("indx"),
                    Some(format!("{}-{}", uuid.to_string(), index.name)),
                    Some(10_000_000),
                ),
            );
        }

        let mut ret_struct = Self {
            data: shard_col,
            indexes: Arc::new(indexes),
            path: table_path,
            uuid,
            table,
            _key_marker: PhantomData,
        };

        ret_struct.init();

        ret_struct
    }

    pub fn init(&mut self) {
        let indexes = self.indexes.clone();
        self.data.temps.set_on_reconcile(Box::new(move |row, pos| {
            let row: T = T::from(row.clone());
            Self::insert_indexes(indexes.clone(), &row, pos);
            Ok(())
        }));
    }

    pub fn insert_indexes(indexes: Arc<CHashMap<String, HashIndex>>, data: &T, pos_index: usize) {
        let table = data.get_table();
        for index in &table.indexes {
            let mut can_index = false;
            let mut composite_key_vals: Vec<(String, String)> = vec![];

            // Loop over each column in the index
            for index_col in &index.members {
                let val = data.get_value(index_col.clone()).unwrap_or(DataValue::Null);

                if !val.is_null() {
                    can_index = true;
                }

                composite_key_vals.push((index_col.clone(), val.to_string()))
            }

            if can_index {
                let index = indexes.get_mut(&(index.name.clone())).unwrap();

                let composite_key = CompositeKey(composite_key_vals);
                let hashed_key = IndexKeySha256::from(composite_key);
                index.insert(hashed_key, pos_index as u64)
            }
        }
    }

    pub fn insert(&self, data: T) {
        self.data.temps.insert_row(data.serialize().unwrap());
    }
}
