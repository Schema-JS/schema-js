use crate::ops::query_ops::{FilterType, QueryOps, QueryVal};
use crate::primitives::Row;
use schemajs_data::index::data::index_shard::IndexShard;
use schemajs_data::index::types::{IndexKey, IndexValue};
use schemajs_data::shard::map_shard::MapShard;
use schemajs_data::shard::shards::data_shard::config::DataShardConfig;
use schemajs_data::shard::shards::data_shard::shard::DataShard;
use schemajs_data::shard::shards::kv::shard::KvShard;
use schemajs_data::shard::Shard;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::table::Table;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

pub struct IndexHandler<T: Row<T>, K: IndexKey, V: IndexValue> {
    pub main: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
    pub data: Arc<IndexShard<K, V>>,
    pub table: Table,
    // Markers
    _key_marker: PhantomData<T>,
}

impl<T: Row<T>, K: IndexKey, V: IndexValue> IndexHandler<T, K, V> {
    pub fn new(
        main: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
        shard: Arc<IndexShard<K, V>>,
        table: Table,
    ) -> Self {
        Self {
            main,
            data: shard,
            table,
            _key_marker: PhantomData,
        }
    }

    pub fn binary_search(&self, shard: &KvShard, query_ops: &QueryOps) -> Vec<T> {
        match query_ops {
            QueryOps::Condition(cond) => {
                let key = &cond.key;
                let data_len = shard.get_last_index();

                let mut left = 0;
                let mut right = data_len + 1;
                while left < right {
                    let mid = (left + right) / 2;
                    let row_mid_val = self.from_indx_to_row(shard, mid);

                    let get_indx_val = row_mid_val
                        .get_value(self.table.get_column(key).unwrap())
                        .unwrap_or_else(|| DataValue::Null);

                    println!("{}", get_indx_val.to_string());

                    match get_indx_val.cmp(&cond.value) {
                        Ordering::Less => left = mid + 1,
                        Ordering::Equal => {
                            return self.expand_search(
                                shard,
                                data_len as usize,
                                mid as usize,
                                cond,
                            );
                        }
                        Ordering::Greater => right = mid,
                    }
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn from_indx_to_row(&self, shard: &KvShard, indx: i64) -> T {
        let row_mid_val = {
            let (k, v, _) = {
                let entry = self
                    .data
                    .get_entry_from_shard(shard, indx as usize)
                    .unwrap();
                let (key_unit, val_unit, el) = self.data.build_entry_from_vec(entry).unwrap();
                self.data.build_kv(key_unit, val_unit, el)
            };

            let index_pos = {
                let raw_val: Vec<u8> = v.into();
                let raw_val_sized: [u8; 8] = raw_val.try_into().unwrap();
                u64::from_le_bytes(raw_val_sized)
            };

            let read_el = self
                .main
                .read()
                .unwrap()
                .get_element(index_pos as usize)
                .unwrap();
            let row = T::from(read_el);

            row
        };
        row_mid_val
    }

    fn expand_search(
        &self,
        shard: &KvShard,
        data_len: usize,
        start_idx: usize,
        query_val: &QueryVal,
    ) -> Vec<T> {
        let mut results = Vec::new();

        let row = self.from_indx_to_row(shard, start_idx as i64);

        results.push(row);

        match query_val.filter_type {
            FilterType::Equal => {
                let mut local = self.expand_in_both_directions(
                    shard,
                    data_len,
                    start_idx,
                    &query_val,
                    Ordering::Equal,
                );
                results.append(&mut local);
            }
            FilterType::GreaterThan | FilterType::GreaterOrEqualTo => {
                if query_val.filter_type.is_greater_than() {
                    results.remove(0);
                }

                let mut local = self.expand_in_one_direction(
                    shard,
                    start_idx + 1,
                    data_len,
                    &query_val,
                    Ordering::Greater,
                );
                results.append(&mut local);
            }
            FilterType::LowerThan | FilterType::LowerOrEqualTo => {
                if query_val.filter_type.is_lower_than() {
                    results.remove(0);
                }

                let mut local =
                    self.expand_in_one_direction(shard, 0, start_idx, &query_val, Ordering::Less);
                results.append(&mut local);
            }
            FilterType::NotEqual => {}
        }

        results
    }

    fn expand_in_both_directions(
        &self,
        shard: &KvShard,
        data_len: usize,
        start_idx: usize,
        query_val: &QueryVal,
        ordering: Ordering,
    ) -> Vec<T> {
        let mut results = vec![];

        let mut left =
            self.expand_in_one_direction(shard, 0, start_idx, query_val, ordering.clone());
        let mut right = self.expand_in_one_direction(
            shard,
            start_idx + 1,
            data_len,
            query_val,
            ordering.clone(),
        );

        results.append(&mut left);
        results.append(&mut right);

        results
    }

    fn expand_in_one_direction(
        &self,
        shard: &KvShard,
        start_idx: usize,
        end_idx: usize,
        query_val: &QueryVal,
        ordering: Ordering,
    ) -> Vec<T> {
        let mut results = vec![];

        for i in start_idx..end_idx {
            if let Some(item) =
                self.match_item(shard, &ordering, &query_val.key, &query_val.value, i)
            {
                results.push(item);
            }
        }

        results
    }

    fn match_item(
        &self,
        shard: &KvShard,
        ordering: &Ordering,
        key: &String,
        query_value: &DataValue,
        i: usize,
    ) -> Option<T> {
        let row_mid_val = self.from_indx_to_row(shard, i as i64);
        let column = self.table.get_column(key.as_str()).unwrap();
        let value = row_mid_val.get_value(column).unwrap();

        if &value.cmp(query_value) == ordering {
            Some(row_mid_val)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ops::index_handler::IndexHandler;
    use crate::ops::query_ops::{FilterType, QueryOps, QueryVal};
    use crate::primitives::Row;
    use crate::row_json::{RowData, RowJson};
    use crate::serializer::RowSerializer;
    use schemajs_data::index::composite_key::CompositeKey;
    use schemajs_data::index::data::index_shard::IndexShard;
    use schemajs_data::index::implementations::hash::hash_index::HashIndex;
    use schemajs_data::index::keys::index_key_sha256::IndexKeySha256;
    use schemajs_data::index::vals::raw_value::RawIndexValue;
    use schemajs_data::index::Index as TraitIndex;
    use schemajs_data::shard::map_shard::MapShard;
    use schemajs_data::shard::shards::data_shard::config::DataShardConfig;
    use schemajs_data::shard::shards::data_shard::shard::DataShard;
    use schemajs_primitives::column::types::{DataTypes, DataValue};
    use schemajs_primitives::column::Column;
    use schemajs_primitives::index::Index;
    use schemajs_primitives::table::Table;
    use std::sync::{Arc, RwLock};
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    pub fn test_index_bin_search() {
        let mut table = Table::new("users");
        table.indexes.push(Index {
            name: "email_indx".to_string(),
            members: vec![String::from("email")],
        });
        let table = table.add_column(Column::new("email", DataTypes::String));

        let (ref_lock, hash_indx, clone_indx, index_handler) =
            setup_index(table, "email_indx".to_string());

        let emails = ["andreespirela@outlook.com", "anotheremail@example.com"];

        for email in emails.iter() {
            let row = RowJson::from(RowData {
                table: "users".to_string(),
                value: serde_json::json!({
                    "_uid": Uuid::new_v4().to_string(),
                    "email": email.to_string(),
                }),
            });

            let indx = ref_lock
                .write()
                .unwrap()
                .insert_row(row.serialize().unwrap());
            let composite_key = CompositeKey(vec![(String::from("email"), email.to_string())]);
            let hashed_key = IndexKeySha256::from(composite_key);
            hash_indx.insert(hashed_key, indx as u64);
        }

        let mr_shard = &clone_indx.data.read().unwrap().current_master_shard;
        // Create the QueryOps variable
        let query_ops = QueryOps::Condition(QueryVal {
            key: "email".to_string(),
            filter_type: FilterType::Equal,
            value: DataValue::String("andreespirela@outlook.com".to_string()),
        });

        let row: Vec<RowJson> = index_handler.binary_search(mr_shard, &query_ops);
        assert_eq!(row.len(), 1);
        println!("{}", row.len());
        let row = row.get(0).unwrap();
        let email_val = row
            .get_value(index_handler.table.get_column("email").unwrap())
            .unwrap();
        assert_eq!(email_val, DataValue::from("andreespirela@outlook.com"));
    }

    #[test]
    pub fn test_index_bin_search_ids() {
        let mut table = Table::new("users");
        table.indexes.push(Index {
            name: "userid_indx".to_string(),
            members: vec![String::from("user_id")],
        });
        let table = table.add_column(Column::new("user_id", DataTypes::Number));

        let (ref_lock, hash_indx, clone_indx, index_handler) =
            setup_index(table, "userid_indx".to_string());

        let uids = [5, 20, 10, 4, 2, 7, 8, 10, 11, 9, 15, 21, 25, 30, 29];

        for uid in uids.iter() {
            let uid = uid.clone();
            let row = RowJson::from(RowData {
                table: "users".to_string(),
                value: serde_json::json!({
                    "_uid": Uuid::new_v4().to_string(),
                    "user_id": uid,
                }),
            });

            let indx = ref_lock
                .write()
                .unwrap()
                .insert_row(row.serialize().unwrap());
            let composite_key = CompositeKey(vec![(String::from("user_id"), uid.to_string())]);
            let hashed_key = IndexKeySha256::from(composite_key);
            hash_indx.insert(hashed_key, indx as u64);
        }

        let mr_shard = &clone_indx.data.read().unwrap().current_master_shard;
        // Create the QueryOps variable
        let query_ops = QueryOps::Condition(QueryVal {
            key: "user_id".to_string(),
            filter_type: FilterType::Equal,
            value: DataValue::Number(serde_json::Number::from(5)),
        });

        let row: Vec<RowJson> = index_handler.binary_search(mr_shard, &query_ops);
        assert_eq!(row.len(), 1);
    }

    fn setup_index(
        table: Table,
        index_name: String,
    ) -> (
        Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
        HashIndex,
        Arc<IndexShard<IndexKeySha256, RawIndexValue>>,
        IndexHandler<RowJson, IndexKeySha256, RawIndexValue>,
    ) {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("./test_cases/data")
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir(temp_dir.clone()).unwrap();
        let data_folder = temp_dir.join("db-data");
        let indx_folder = data_folder.join("indx");

        std::fs::create_dir(data_folder.clone()).unwrap();
        std::fs::create_dir(indx_folder.clone()).unwrap();

        let map_shard = MapShard::<DataShard, DataShardConfig>::new(
            data_folder.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(5),
            },
        );

        let ref_lock = Arc::new(RwLock::new(map_shard));
        let hash_indx = HashIndex::new_from_path(indx_folder, Some(index_name), None);

        let clone_indx = hash_indx.index.clone();

        let index_handler = IndexHandler::new(ref_lock.clone(), clone_indx.clone(), table);
        (ref_lock, hash_indx, clone_indx, index_handler)
    }
}
