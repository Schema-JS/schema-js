pub mod metadata;

use crate::column::types::DataTypes;
use crate::column::Column;
use crate::index::Index;
use crate::table::metadata::TableMetadata;
use schemajs_index::index_type::IndexType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub indexes: Vec<Index>,
    pub primary_key: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: TableMetadata,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Table {
            name: name.to_string(),
            columns: HashMap::from([("_uid".to_string(), Self::get_internal_uid())]),
            metadata: Default::default(),
            primary_key: "_uid".to_string(),
            indexes: vec![Self::get_internal_uid_index()],
        }
    }

    pub fn init(&mut self) {
        self.columns
            .insert("_uid".to_string(), Self::get_internal_uid());
        self.indexes.push(Self::get_internal_uid_index());
    }

    pub fn get_internal_uid() -> Column {
        Column::new("_uid", DataTypes::Uuid)
            .set_required(true)
            .set_primary_key(true)
    }

    fn get_internal_uid_index() -> Index {
        Index {
            name: "uidindx".to_string(),
            members: vec!["_uid".to_string()],
            index_type: IndexType::Hash,
        }
    }

    // TODO: Handle known index
    pub fn add_index(mut self, index: Index) -> Self {
        self.indexes.push(index);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        if column.primary_key {
            if self.primary_key == "_uid".to_string() {
                self.primary_key = column.name.clone();
            } else {
                todo!("Handle");
            }
        }

        self.columns.insert(column.name.clone(), column);
        self
    }

    pub fn get_column(&self, column_name: &str) -> Option<&Column> {
        self.columns.get(column_name)
    }

    pub fn list_columns(&self) -> Vec<&String> {
        self.columns.keys().collect()
    }
}
