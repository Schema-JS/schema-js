use schemajs_primitives::database::Database;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub struct SchemeJsEngine {
    pub databases: Vec<Database>,
}

pub type ArcSchemeJsEngine = Arc<RefCell<SchemeJsEngine>>;

impl SchemeJsEngine {
    pub fn new() -> Self {
        Self { databases: vec![] }
    }

    pub fn find_by_name(&mut self, name: String) -> Option<&mut Database> {
        self.databases.iter_mut().find(|i| i.name == name)
    }

    pub fn add_database(&mut self, name: &str) {
        self.databases.push(Database {
            name: name.to_string(),
            tables: HashMap::new(),
        })
    }
}
