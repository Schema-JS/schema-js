use crate::managers::single::query_shard_entry::QueryShardEntry;
use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_primitives::table::Table;
use std::hash::Hash;
use uuid::Uuid;

pub struct QueryShard<T: Row<T> + Hash> {
    pub table_shards: CHashMap<String, QueryShardEntry<T>>,
    pub scheme_name: String,
    pub scheme_uuid: String,
    pub uuid: Uuid,
}

impl<T: Row<T> + Hash> QueryShard<T> {
    pub fn new(scheme_name: String, scheme_uuid: String) -> Self {
        Self {
            table_shards: CHashMap::new(),
            scheme_name,
            scheme_uuid,
            uuid: Uuid::new_v4(),
        }
    }

    pub fn insert(&self, data: T) {
        let table = data.get_table();
        let table_name = table.name.clone();
        if let Some(entry) = self.table_shards.get(&table_name) {
            entry.data.temps.insert_row(data.serialize().unwrap());
        } else {
            let shard = QueryShardEntry::<T>::new(
                format!("{}_{}", self.scheme_name, self.scheme_uuid),
                table_name.clone(),
                table.clone(),
            );

            shard.insert(data);

            self.table_shards.insert(table_name, shard);
        }
    }
}
