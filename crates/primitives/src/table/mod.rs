pub mod metadata;

use crate::column::types::DataTypes;
use crate::column::Column;
use crate::index::Index;
use crate::table::metadata::TableMetadata;
use schemajs_index::index_type::IndexType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub indexes: Vec<Index>,
    pub primary_key: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: TableMetadata,
}

static INTERNAL_COLUMNS: LazyLock<HashMap<String, Column>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "_uid".to_string(),
        Column::new("_uid", DataTypes::Uuid)
            .set_required(true)
            .set_primary_key(true),
    );

    map
});

static UID_INDEX: LazyLock<Index> = LazyLock::new(|| Index {
    name: "uidindx".to_string(),
    members: vec!["_uid".to_string()],
    index_type: IndexType::Hash,
});

static RESERVED_COLUMN_NAMES: [&str; 1] = ["_uid"];

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
        self.raw_add_column(Self::get_internal_uid().clone());

        for (col_name, col) in &self.columns {
            if RESERVED_COLUMN_NAMES.contains(&col_name.as_str()) {
                continue;
            }

            if col.default_index.unwrap_or(false) {
                self.indexes.push(Index {
                    name: format!("{}_indx", col_name),
                    members: vec![col_name.to_string()],
                    index_type: IndexType::Hash,
                });
            }
        }

        self.raw_add_index(Self::get_internal_uid_index().clone());
    }

    pub fn get_internal_uid<'a>() -> &'a Column {
        (&*INTERNAL_COLUMNS).get("_uid").unwrap()
    }

    fn get_internal_uid_index<'a>() -> &'a Index {
        &*UID_INDEX
    }

    fn raw_add_index(&mut self, index: Index) {
        let exists = self.indexes.iter().any(|i| i.name == index.name);
        if !exists {
            self.indexes.push(index);
        }
    }

    pub fn add_index(mut self, index: Index) -> Self {
        self.raw_add_index(index);
        self
    }

    fn raw_add_column(&mut self, column: Column) {
        if column.primary_key {
            self.primary_key = column.name.clone();
        }

        if !self.columns.contains_key(&column.name) {
            self.columns.insert(column.name.clone(), column);
        }
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.raw_add_column(column);
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
