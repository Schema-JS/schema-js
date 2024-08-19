use crate::errors::QueryError;
use crate::managers::single::query_shard_entry::QueryShardEntry;
use crate::primitives::Row;
use chashmap::CHashMap;
use schemajs_primitives::table::Table;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct QueryShard<T: Row<T>> {
    pub table_shards: RwLock<HashMap<String, QueryShardEntry<T>>>,
    pub scheme_name: String,
    pub scheme_uuid: String,
    pub uuid: Uuid,
}

impl<T: Row<T>> QueryShard<T> {
    pub fn new(scheme_name: String, scheme_uuid: String) -> Self {
        Self {
            table_shards: RwLock::new(HashMap::new()),
            scheme_name,
            scheme_uuid,
            uuid: Uuid::new_v4(),
        }
    }

    pub fn reconcile_all(&self) {
        for (_, shard_entry) in self.table_shards.read().unwrap().iter() {
            shard_entry.data.temps.write().unwrap().reconcile_all();
        }
    }

    pub fn insert(&self, table: Table, data: T) -> Result<Uuid, QueryError> {
        let uuid = data
            .get_value(&Table::get_internal_uid())
            .ok_or(QueryError::UnknownUid)?;

        let serialized_value = data
            .serialize()
            .map_err(|e| QueryError::InvalidSerialization)?;

        let reader = self.table_shards.read().unwrap();

        let maybe_table = { reader.get(&table.name) };

        if let Some(entry) = maybe_table {
            entry
                .data
                .temps
                .write()
                .unwrap()
                .insert_row(serialized_value)?;
        } else {
            let shard = QueryShardEntry::<T>::new(
                format!("{}_{}", self.scheme_name, self.scheme_uuid),
                table.name.clone(),
                table.clone(),
            );

            shard
                .data
                .temps
                .write()
                .unwrap()
                .insert_row(serialized_value)?;

            drop(reader);

            self.table_shards
                .write()
                .unwrap()
                .insert(table.name.clone(), shard);
        }

        let uuid = uuid.as_uuid().unwrap().clone();
        Ok(uuid)
    }
}
