use crate::engine_db::EngineDb;
use crate::engine_table::EngineTable;
use crate::utils::fs::is_js_or_ts;
use anyhow::bail;
use deno_core::{ModuleId, ModuleSpecifier};
use schemajs_dirs::create_scheme_js_folder;
use schemajs_primitives::table::Table;
use std::cell::RefCell;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use walkdir::WalkDir;

pub type ArcSchemeJsEngine = Arc<RefCell<SchemeJsEngine>>;

pub struct SchemeJsEngine {
    pub databases: Vec<EngineDb>,
    pub data_path_dir: Option<PathBuf>,
}

impl SchemeJsEngine {
    pub fn new(data_path: Option<PathBuf>) -> Self {
        create_scheme_js_folder(data_path.clone());

        Self {
            databases: vec![],
            data_path_dir: data_path,
        }
    }

    pub fn load_database_schema(
        &mut self,
        path: &PathBuf,
    ) -> anyhow::Result<(String, Vec<ModuleSpecifier>)> {
        if !path.exists() {
            bail!(
                "Trying to access a database schema that does not exist: {}",
                path.to_string_lossy()
            );
        }

        let schema_name = path.file_name().unwrap().to_str().unwrap();

        {
            self.add_database(schema_name);
        }

        let table_path = path.join("tables").canonicalize()?;
        let table_walker = WalkDir::new(table_path).into_iter().filter_map(|e| e.ok());

        let mut table_specifiers = vec![];

        for table_file in table_walker {
            if is_js_or_ts(&table_file) {
                let url = ModuleSpecifier::from_file_path(table_file.path()).unwrap();
                table_specifiers.push(url);
            }
        }

        Ok((schema_name.to_string(), table_specifiers))
    }

    pub fn register_tables(&mut self, schema_name: &str, loaded_tables: Vec<Table>) {
        let data_path_dir = self.data_path_dir.clone();
        let mut db = self.find_by_name(schema_name.to_string()).unwrap();
        for table in loaded_tables {
            db.add_table(EngineTable::new(data_path_dir.clone(), schema_name, table));
        }
    }

    pub fn find_by_name(&mut self, name: String) -> Option<&mut EngineDb> {
        self.databases.iter_mut().find(|i| i.name == name)
    }

    pub fn find_by_name_ref(&self, name: String) -> Option<&EngineDb> {
        self.databases.iter().find(|i| i.name == name)
    }

    pub fn add_database(&mut self, name: &str) {
        self.databases
            .push(EngineDb::new(self.data_path_dir.clone(), name))
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
    use std::sync::{Arc, RwLock};
    use std::thread;

    #[tokio::test]
    pub async fn test_db_engine() {
        let db_engine = Arc::new(RwLock::new(SchemeJsEngine::new(None)));

        // Add database
        {
            let mut writer = db_engine.write().unwrap();
            writer.add_database("rust-test");
        } // Release the write lock

        {
            {
                let mut reader = db_engine.read().unwrap();
                let db = reader.find_by_name_ref("rust-test".to_string()).unwrap();

                assert_eq!(db.db_folder.exists(), true);
            }

            {
                let mut cols: HashMap<String, Column> = HashMap::new();
                cols.insert(
                    "id".to_string(),
                    Column {
                        name: "id".to_string(),
                        data_type: DataTypes::String,
                        default_value: None,
                        required: false,
                        comment: None,
                    },
                );

                let table = Table {
                    name: "users".to_string(),
                    columns: cols,
                    module_id: None,
                };

                let mut writer = db_engine.write().unwrap();
                let mut db = writer.find_by_name("rust-test".to_string()).unwrap();
                db.add_table(EngineTable::new(None, "rust-test", table));
            }

            {
                let mut reader = db_engine.read().unwrap();
                let db = reader.find_by_name_ref("rust-test".to_string()).unwrap();
                let users = db.get_table_ref("users").unwrap();
                assert_eq!(users.tbl_folder.exists(), true);
            }
        }

        let arc = db_engine.clone();

        let ref_shard1 = Arc::clone(&arc);
        let thread_1 = thread::spawn(move || {
            let mut writer = ref_shard1.write().unwrap();
            let table = writer
                .find_by_name("rust-test".to_string())
                .unwrap()
                .get_table("users")
                .unwrap();
            table.temp_shards.insert_row(b"1".to_vec());
        });

        let ref_shard2 = Arc::clone(&arc);
        let thread_2 = thread::spawn(move || {
            let mut writer = ref_shard2.write().unwrap();
            let table = writer
                .find_by_name("rust-test".to_string())
                .unwrap()
                .get_table("users")
                .unwrap();
            table.temp_shards.insert_row(b"2".to_vec());
        });

        thread_1.join().unwrap();
        thread_2.join().unwrap();

        // Assuming `temp_shards` is part of `EngineTable` and is a `RwLock<HashMap<String, Shard>>`
        {
            let mut reader = db_engine.write().unwrap();
            let mut db = reader.find_by_name("rust-test".to_string()).unwrap();
            let users = db.get_table("users").unwrap();
            let temp_shards = &users.temp_shards;

            let reader = temp_shards.temp_shards.read().unwrap();
            let shards = reader.iter().next().unwrap();
            let first_item = shards.read_item_from_index(0).unwrap();
            let second_item = shards.read_item_from_index(1).unwrap();

            let mut items: Vec<Vec<u8>> = vec![first_item, second_item];

            items.sort();

            assert_eq!(items, vec![b"1".to_vec(), b"2".to_vec()]);
        }
    }
}
