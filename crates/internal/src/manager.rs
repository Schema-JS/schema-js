use crate::auth::auth_manager::AuthManager;
use crate::get_internal_tables;
use schemajs_config::SchemeJsConfig;
use schemajs_engine::engine::SchemeJsEngine;
use std::sync::{Arc, RwLock};

pub struct InternalManager {
    _engine: Arc<RwLock<SchemeJsEngine>>,
    auth_manager: Arc<AuthManager>,
}

impl InternalManager {
    pub fn new(engine: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            _engine: engine.clone(),
            auth_manager: Arc::new(AuthManager::new(engine)),
        }
    }

    pub fn get_config(&self) -> Arc<SchemeJsConfig> {
        self._engine.read().unwrap().config.clone()
    }

    pub fn init(&self) {
        {
            let mut writer = self._engine.write().unwrap();
            let default_scheme_name = writer.config.global.default_scheme.clone();
            {
                if !writer.contains_db(&default_scheme_name) {
                    writer.add_database(&default_scheme_name);
                }
            }
        }

        // Load Internal tables
        let dbs = {
            let read_engine = self._engine.read().unwrap();

            let db_names: Vec<String> = read_engine
                .databases
                .iter()
                .map(|e| e.name.clone())
                .collect();
            for schema_name in &db_names {
                read_engine.register_tables(schema_name, get_internal_tables());
            }

            db_names
        };

        {
            for db_name in dbs {
                self.auth_manager.init_default_user(&db_name);
            }
        }
    }

    pub fn engine(&self) -> Arc<RwLock<SchemeJsEngine>> {
        self._engine.clone()
    }

    pub fn auth_manager(&self) -> Arc<AuthManager> {
        self.auth_manager.clone()
    }
}
