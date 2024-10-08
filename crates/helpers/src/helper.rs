use dashmap::DashMap;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Serialize, Deserialize, EnumAsInner, Debug, Clone)]
pub enum HelperType {
    CustomQuery,
    InsertHook,
}

#[derive(Debug)]
pub struct Helper {
    pub identifier: String,
    pub internal_type: HelperType,
    pub func: deno_core::v8::Global<deno_core::v8::Function>,
}

#[derive(Debug, Default)]
pub struct SjsHelpersContainer(pub Vec<Arc<Helper>>);

impl SjsHelpersContainer {
    pub fn new(data: Vec<Arc<Helper>>) -> Self {
        Self(data)
    }
}

/// DashMap<DbName, DashMap<TableName, HelperContainer>>
pub struct SjsTableHelpers(pub DashMap<String, DashMap<String, SjsHelpersContainer>>);

impl SjsTableHelpers {
    pub fn find_custom_query_helper(
        &self,
        db_name: &str,
        table: &str,
        identifier: &str,
    ) -> Option<Arc<Helper>> {
        match self.0.get(db_name) {
            None => None,
            Some(val) => {
                let helper = val.get(table).map(|e| {
                    e.0.iter()
                        .find(|e| e.identifier == identifier)
                        .map(|e| e.clone())
                });

                helper.unwrap_or_else(|| None)
            }
        }
    }

    pub fn find_hook_helper(
        &self,
        db_name: &str,
        table: &str,
        hook: HelperType,
    ) -> Option<Vec<Arc<Helper>>> {
        match self.0.get(db_name) {
            None => None,
            Some(val) => match hook {
                HelperType::InsertHook => {
                    let helper: Option<Vec<Arc<Helper>>> = val.get(table).map(|e| {
                        e.0.iter()
                            .filter(|e| e.internal_type.is_insert_hook())
                            .map(|e| e.clone())
                            .collect()
                    });

                    helper
                }
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct HelperDbContext {
    pub db: Option<String>,
    pub table: Option<String>,
}

#[derive(EnumAsInner, Debug, Clone)]
pub enum HelperCall {
    CustomQuery {
        db_ctx: HelperDbContext,
        identifier: String,
        req: Value,
        response: UnboundedSender<Value>,
    },
    InsertHook {
        db_ctx: HelperDbContext,
        rows: Vec<Value>,
    },
}
