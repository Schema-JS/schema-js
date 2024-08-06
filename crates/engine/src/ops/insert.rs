use crate::engine::SchemeJsEngine;
use deno_core::{op2, serde_json, OpState};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[op2(async)]
pub async fn op_engine_insert_row(
    state: Rc<RefCell<OpState>>,
    #[string] db_name: String,
    #[string] table_name: String,
    #[serde] row: serde_json::Value,
) {
    let mut mut_state = state.borrow_mut();
    let state = mut_state.borrow_mut::<Arc<SchemeJsEngine>>().clone();

    let table = {
        let db = state.find_by_name_ref(db_name.clone()).unwrap();
        db.get_table_ref(table_name.as_str()).unwrap()
    };

    let insert = table.insert_row(row);

    insert.unwrap();
}
