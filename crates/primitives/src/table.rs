use crate::column::Column;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Table {
            name: name.to_string(),
            columns: HashMap::new(),
        }
    }

    pub fn add_column(mut self, column: Column) -> Self {
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
