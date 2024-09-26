use crate::auth::auth_manager::AuthManager;
use crate::get_internal_tables;
use schemajs_engine::engine::SchemeJsEngine;
use std::sync::{Arc, RwLock};

pub struct InternalManager {
    engine: Arc<RwLock<SchemeJsEngine>>,
    auth_manager: Arc<AuthManager>,
}

impl InternalManager {
    pub fn new(engine: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            engine: engine.clone(),
            auth_manager: Arc::new(AuthManager::new(engine)),
        }
    }

    pub fn init(&self) {
        {
            let mut writer = self.engine.write().unwrap();
            let default_workspace = writer.config.default.clone().unwrap();
            let default_scheme_name = &default_workspace.scheme_name;
            {
                writer.add_database(default_scheme_name);
            }

            // Load Internal tables
            {
                let db_names: Vec<String> =
                    writer.databases.iter().map(|e| e.name.clone()).collect();
                for schema_name in db_names {
                    writer.register_tables(&schema_name, get_internal_tables());
                }
            }
        }

        {
            self.auth_manager.init_default_user();
        }
    }

    pub fn auth_manager(&self) -> Arc<AuthManager> {
        self.auth_manager.clone()
    }
}
