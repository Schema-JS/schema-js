use crate::auth::types::VerifyUserArgs;
use crate::users::user::{create_user, INTERNAL_USER_TABLE, INTERNAL_USER_TABLE_NAME};
use schemajs_engine::engine::SchemeJsEngine;
use schemajs_engine::engine_db::EngineDb;
use schemajs_primitives::column::types::DataValue;
use schemajs_query::ops::query_ops::{QueryOps, QueryVal};
use schemajs_query::row::Row;
use schemajs_query::row_json::{RowData, RowJson};
use std::sync::{Arc, RwLock};

pub struct AuthManager {
    engine: Arc<RwLock<SchemeJsEngine>>,
}

impl AuthManager {
    pub fn new(engine: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self { engine }
    }

    pub fn verify_user(&self, args: VerifyUserArgs) -> bool {
        let engine = self.engine.read().unwrap();
        let table = &*INTERNAL_USER_TABLE;
        if let Some(db) = engine.find_by_name_ref(args.scheme_name) {
            let u = Self::search_user(db, &args.identifier);

            if let Some(user) = u {
                let hashed_password = user
                    .get_value(table.get_column("hashed_password").unwrap())
                    .unwrap()
                    .to_string();
                return bcrypt::verify(args.password, hashed_password.as_str()).unwrap();
            }
        }

        false
    }

    pub fn init_default_user(&self) {
        let mut engine = self.engine.write().unwrap();
        let config = engine.config.clone();
        let default_scheme = config.default.clone().unwrap();
        let default_scheme_name = default_scheme.scheme_name.clone();
        // Load default user
        let db = engine
            .find_by_name(default_scheme_name.to_string())
            .unwrap();

        let scheme_username = default_scheme.username.clone();

        let search_users = Self::search_user(db, &scheme_username);

        if search_users.is_none() {
            let _ = db
                .query_manager
                .raw_insert(
                    RowJson::from(RowData {
                        table: INTERNAL_USER_TABLE_NAME.to_string(),
                        value: serde_json::to_value(create_user(
                            scheme_username,
                            default_scheme.password.clone(),
                            true,
                            true,
                            vec![],
                        ))
                        .unwrap(),
                    }),
                    true,
                )
                .unwrap();
        }
    }

    fn search_user(db: &EngineDb, scheme_username: &String) -> Option<RowJson> {
        let users = db
            .query_manager
            .search_manager
            .search(
                INTERNAL_USER_TABLE_NAME.to_string(),
                &QueryOps::And(vec![QueryOps::Condition(QueryVal {
                    key: "identifier".to_string(),
                    filter_type: "=".to_string(),
                    value: DataValue::String(scheme_username.clone()),
                })]),
            )
            .unwrap();

        if !users.is_empty() {
            Some(users[0].clone())
        } else {
            None
        }
    }
}
