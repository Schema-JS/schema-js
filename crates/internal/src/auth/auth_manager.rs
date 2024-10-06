use crate::auth::types::{UserContext, VerifyUserArgs};
use crate::users::user::{create_user, User, INTERNAL_USER_TABLE, INTERNAL_USER_TABLE_NAME};
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use schemajs_engine::engine::SchemeJsEngine;
use schemajs_engine::engine_db::EngineDb;
use schemajs_primitives::column::types::DataValue;
use schemajs_query::ops::query_ops::{QueryOps, QueryVal};
use schemajs_query::row::Row;
use schemajs_query::row_json::{RowData, RowJson};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub struct AuthManager {
    engine: Arc<RwLock<SchemeJsEngine>>,
    authenticated_users: DashMap<String, Arc<UserContext>>,
}

impl AuthManager {
    pub fn new(engine: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            engine,
            authenticated_users: DashMap::new(),
        }
    }

    pub fn check_token(&self, uuid: &str) -> Result<Arc<UserContext>, ()> {
        let reader = self.authenticated_users.get(uuid);
        if let Some(ctx) = reader {
            Ok(ctx.value().clone())
        } else {
            Err(())
        }
    }

    pub fn authenticate(&self, args: VerifyUserArgs) -> Result<Uuid, ()> {
        let user = self.verify_user(args);

        if let Some(user) = user {
            let token = Uuid::new_v4();
            let ctx = UserContext::new(user);
            self.authenticated_users
                .insert(token.to_string(), Arc::new(ctx));
            return Ok(token);
        }

        Err(())
    }

    pub fn verify_user(&self, args: VerifyUserArgs) -> Option<User> {
        let engine = self.engine.read().unwrap();
        let table = &*INTERNAL_USER_TABLE;
        if let Some(db) = engine.find_by_name_ref(&args.scheme_name) {
            let u = Self::search_user(db, &args.identifier);

            if let Some(user) = u {
                let hashed_password = user
                    .get_value(table.get_column("hashed_password").unwrap())
                    .unwrap()
                    .to_string();

                let is_password_correct =
                    bcrypt::verify(args.password, hashed_password.as_str()).unwrap();
                if is_password_correct {
                    return Some(User {
                        identifier: args.identifier,
                        hashed_password,
                        created_at: user
                            .get_value(table.get_column("created_at").unwrap())
                            .unwrap()
                            .as_number()
                            .unwrap()
                            .as_u64()
                            .unwrap(),
                        updated_at: user
                            .get_value(table.get_column("updated_at").unwrap())
                            .unwrap()
                            .as_number()
                            .unwrap()
                            .as_u64()
                            .unwrap(),
                        is_admin: user
                            .get_value(table.get_column("is_admin").unwrap())
                            .unwrap()
                            .as_boolean()
                            .unwrap_or_else(|| &false)
                            .clone(),
                        is_super_admin: user
                            .get_value(table.get_column("is_super_admin").unwrap())
                            .unwrap()
                            .as_boolean()
                            .unwrap_or_else(|| &false)
                            .clone(),
                        roles: vec![],
                        scheme: args.scheme_name,
                    });
                }
            }
        }

        None
    }

    pub fn init_default_user(&self) {
        let mut engine = self.engine.write().unwrap();
        let config = engine.config.clone();
        let default_scheme_name = config.global.default_scheme.clone();
        let default_auth = config.global.default_auth.clone();

        // Load default user
        let db = engine.find_by_name_ref(&default_scheme_name).unwrap();

        let search_users = Self::search_user(db, &default_auth.username);

        if search_users.is_none() {
            let tbl = db
                .query_manager
                .get_table(INTERNAL_USER_TABLE_NAME)
                .unwrap();
            let user_row = RowJson::from_json(
                serde_json::to_value(create_user(
                    default_auth.username.clone(),
                    default_auth.password.clone(),
                    true,
                    true,
                    vec![],
                    default_scheme_name.to_string(),
                ))
                .unwrap(),
                tbl,
            )
            .unwrap();
            let _ = db.query_manager.raw_insert(&mut [user_row], true).unwrap();
        }
    }

    fn search_user(db: &EngineDb, scheme_username: &String) -> Option<RowJson> {
        let users = db
            .query_manager
            .search_manager
            .search(
                INTERNAL_USER_TABLE_NAME,
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
