pub mod metadata;

use crate::column::types::DataTypes;
use crate::column::Column;
use crate::index::Index;
use crate::table::metadata::TableMetadata;
use schemajs_index::index_type::IndexType;
use serde::{Deserialize, Serialize};
use std::cell::LazyCell;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub indexes: Vec<Index>,
    pub primary_key: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: TableMetadata,
}

static UID_COL: LazyLock<Column> = LazyLock::new(|| {
    Column::new("_uid", DataTypes::Uuid)
        .set_required(true)
        .set_primary_key(true)
});

static UID_INDEX: LazyLock<Index> = LazyLock::new(|| Index {
    name: "uidindx".to_string(),
    members: vec!["_uid".to_string()],
    index_type: IndexType::Hash,
});

impl Table {
    pub fn new(name: &str) -> Self {
        Table {
            name: name.to_string(),
            columns: HashMap::from([("_uid".to_string(), Self::get_internal_uid().clone())]),
            metadata: Default::default(),
            primary_key: "_uid".to_string(),
            indexes: vec![Self::get_internal_uid_index().clone()],
        }
    }

    pub fn init(&mut self) {
        self.columns
            .insert("_uid".to_string(), Self::get_internal_uid().clone());
        self.indexes.push(Self::get_internal_uid_index().clone());
    }

    pub fn get_internal_uid<'a>() -> &'a Column {
        &*UID_COL
    }

    fn get_internal_uid_index<'a>() -> &'a Index {
        &*UID_INDEX
    }

    pub fn add_index(mut self, index: Index) -> Self {
        self.indexes.push(index);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        if column.primary_key {
            self.primary_key = column.name.clone();
        }

        if column.default_index.unwrap() {
            self.indexes.push(Index {
                name: format!("{}_indx", &column.name),
                members: vec![column.name.to_string()],
                index_type: IndexType::Hash,
            });
        }

        self.columns.insert(column.name.clone(), column);
        self
    }

    pub fn set_internal(mut self, internal: bool) -> Self {
        self.metadata.internal = internal;
        self
    }

    pub fn get_column(&self, column_name: &str) -> Option<&Column> {
        self.columns.get(column_name)
    }

    pub fn list_columns(&self) -> Vec<&String> {
        self.columns.keys().collect()
    }
}
