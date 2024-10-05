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

pub struct SjsTableHelpers(pub DashMap<String, SjsHelpersContainer>);

impl SjsTableHelpers {
    pub fn find_custom_query_helper(&self, table: &str, identifier: &str) -> Option<Arc<Helper>> {
        match self.0.get(table) {
            None => None,
            Some(val) => {
                let helper = val
                    .0
                    .iter()
                    .find(|e| e.identifier == identifier)
                    .map(|e| e.clone());
                helper
            }
        }
    }
}

#[derive(EnumAsInner, Debug, Clone)]
pub enum HelperCall {
    CustomQuery {
        table: String,
        identifier: String,
        req: Value,
        response: UnboundedSender<Value>,
    },
    InsertHook {
        rows: Vec<Value>,
    },
}
