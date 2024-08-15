use crate::managers::single::query_shard_entry::QueryShardEntry;
use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_primitives::table::Table;
use std::hash::Hash;
use uuid::Uuid;

#[derive(Debug)]
pub struct QueryShard<T: Row<T>> {
    pub table_shards: CHashMap<String, QueryShardEntry<T>>,
    pub scheme_name: String,
    pub scheme_uuid: String,
    pub uuid: Uuid,
}

impl<T: Row<T>> QueryShard<T> {
    pub fn new(scheme_name: String, scheme_uuid: String) -> Self {
        Self {
            table_shards: CHashMap::new(),
            scheme_name,
            scheme_uuid,
            uuid: Uuid::new_v4(),
        }
    }

    pub fn insert(&self, table: Table, data: T) -> Uuid {
        let uuid = data.get_value(&Table::get_internal_uid()).unwrap();

        if let Some(entry) = self.table_shards.get(&table.name) {
            entry.data.temps.insert_row(data.serialize().unwrap());
        } else {
            let shard = QueryShardEntry::<T>::new(
                format!("{}_{}", self.scheme_name, self.scheme_uuid),
                table.name.clone(),
                table.clone(),
            );

            self.table_shards.insert(table.name.clone(), shard);
        }

        let uuid = uuid.as_uuid().unwrap().clone();
        uuid
    }
}
