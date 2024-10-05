use schemajs_dirs::create_scheme_js_db;
use schemajs_helpers::helper::HelperCall;
use schemajs_primitives::table::Table;
use schemajs_query::managers::single::SingleQueryManager;
use schemajs_query::row_json::RowJson;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct EngineDb {
    pub db_folder: PathBuf,
    pub query_manager: Arc<SingleQueryManager<RowJson>>,
    pub name: String,
    helper_tx: Sender<HelperCall>,
}

impl EngineDb {
    pub fn new(base_path: Option<PathBuf>, name: &str, helper_tx: Sender<HelperCall>) -> Self {
        let db_folder = create_scheme_js_db(base_path.clone(), name);

        let mut query_manager = SingleQueryManager::new(name.to_string(), helper_tx.clone());
        query_manager.data_path = base_path.clone();

        EngineDb {
            name: name.to_string(),
            db_folder,
            query_manager: Arc::new(query_manager),
            helper_tx,
        }
    }

    pub async fn call_helper(&self, call: HelperCall) -> Result<(), SendError<HelperCall>> {
        self.helper_tx.send(call).await
    }

    pub fn add_table(&self, table: Table) {
        self.query_manager.register_table(table);
    }
}
