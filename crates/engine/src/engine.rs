use crate::engine_db::EngineDb;
use schemajs_dirs::create_scheme_js_folder;
use std::cell::RefCell;
use std::sync::Arc;

pub type ArcSchemeJsEngine = Arc<RefCell<SchemeJsEngine>>;

pub struct SchemeJsEngine {
    pub databases: Vec<EngineDb>,
}

impl SchemeJsEngine {
    pub fn new() -> Self {
        create_scheme_js_folder();

        Self { databases: vec![] }
    }

    pub fn find_by_name(&mut self, name: String) -> Option<&mut EngineDb> {
        self.databases.iter_mut().find(|i| i.name == name)
    }

    pub fn add_database(&mut self, name: &str) {
        self.databases.push(EngineDb::new(name))
    }
}

#[cfg(test)]
mod test {
    use crate::engine::SchemeJsEngine;
    use crate::engine_table::EngineTable;
    use schemajs_primitives::column::Column;
    use schemajs_primitives::table::Table;
    use schemajs_primitives::types::DataTypes;
    use std::collections::HashMap;

    #[tokio::test]
    pub async fn test_db_engine() {
        let mut db_engine = SchemeJsEngine::new();
        db_engine.add_database("rust-test");

        let db = db_engine.find_by_name("rust-test".to_string()).unwrap();
        assert_eq!(db.db_folder.exists(), true);

        let mut cols: HashMap<String, Column> = HashMap::new();
        cols.insert(
            "id".to_string(),
            Column {
                name: "id".to_string(),
                data_type: DataTypes::String,
                default_value: None,
                comment: None,
            },
        );

        let table = Table {
            name: "users".to_string(),
            columns: cols,
            module_id: None,
        };

        db.add_table(EngineTable::new("rust-test", table));

        let users = db.get_table("users").unwrap();
        assert_eq!(users.tbl_folder.exists(), true);

        users.temp_shards.insert_row(b"1".to_vec());
    }
}
