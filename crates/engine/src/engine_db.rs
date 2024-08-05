use crate::engine_table::EngineTable;
use schemajs_dirs::create_scheme_js_db;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub struct EngineDb {
    pub db_folder: PathBuf,
    pub name: String,
    pub tables: HashMap<String, EngineTable>,
}

impl EngineDb {
    pub fn new(name: &str) -> Self {
        let db_folder = create_scheme_js_db(name);

        EngineDb {
            name: name.to_string(),
            tables: HashMap::new(),
            db_folder,
        }
    }

    pub fn add_table(&mut self, table: EngineTable) {
        self.tables.insert(table.prim_table.name.clone(), table);
    }

    pub fn get_table(&mut self, name: &str) -> Option<&mut EngineTable> {
        self.tables.get_mut(name)
    }

    pub fn get_table_ref(&self, name: &str) -> Option<&EngineTable> {
        self.tables.get(name)
    }
}
