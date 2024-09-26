use crate::engine::SchemeJsEngine;
use deno_core::{op2, serde_json, OpState};
use schemajs_query::errors::QueryError;
use schemajs_query::row_json::{RowData, RowJson};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[op2(async)]
#[serde]
pub async fn op_engine_insert_row(
    state: Rc<RefCell<OpState>>,
    #[string] db_name: String,
    #[string] table_name: String,
    #[serde] mut row: serde_json::Value,
) -> Result<Uuid, QueryError> {
    let mut mut_state = state.borrow_mut();
    let state = mut_state
        .borrow_mut::<Arc<RwLock<SchemeJsEngine>>>()
        .clone();

    let query_manager = {
        let read_engine = state.read().unwrap();
        let db = read_engine.find_by_name_ref(db_name.clone()).unwrap();
        db.query_manager.clone()
    };

    // Check if the Value is an object
    if let serde_json::Value::Object(ref mut obj) = row {
        // Insert a new field
        obj.insert(
            "_uid".to_string(),
            serde_json::Value::String(Uuid::new_v4().to_string()),
        );
    }

    let insert = query_manager.insert(RowJson::from(RowData {
        table: table_name,
        value: row,
    }));

    if insert.is_err() {
        println!("Error");
    }

    insert
}
