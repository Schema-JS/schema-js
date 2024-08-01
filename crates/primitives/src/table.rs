use crate::column::Column;
use deno_core::ModuleId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub module_id: Option<ModuleId>,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Table {
            name: name.to_string(),
            columns: HashMap::new(),
            module_id: None,
        }
    }

    pub fn set_module_id(&mut self, module_id: ModuleId) {
        self.module_id = Some(module_id);
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
