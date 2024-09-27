use schemajs_dirs::create_scheme_js_db;
use schemajs_primitives::table::Table;
use schemajs_query::managers::single::SingleQueryManager;
use schemajs_query::row_json::RowJson;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct EngineDb {
    pub db_folder: PathBuf,
    pub query_manager: Arc<SingleQueryManager<RowJson>>,
    pub name: String,
}

impl EngineDb {
    pub fn new(base_path: Option<PathBuf>, name: &str) -> Self {
        let db_folder = create_scheme_js_db(base_path.clone(), name);

        let mut query_manager = SingleQueryManager::new(name.to_string());
        query_manager.data_path = base_path.clone();

        EngineDb {
            name: name.to_string(),
            db_folder,
            query_manager: Arc::new(query_manager),
        }
    }

    pub fn add_table(&self, table: Table) {
        self.query_manager.register_table(table);
    }
}
