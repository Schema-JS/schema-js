use crate::table::Table;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Database {
    pub name: String,
    pub tables: HashMap<String, Table>,
}

impl Database {
    pub fn new(name: &str) -> Self {
        Database {
            name: name.to_string(),
            tables: HashMap::new(),
        }
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }
}
