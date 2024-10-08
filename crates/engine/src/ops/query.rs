use crate::engine::SchemeJsEngine;
use deno_core::{op2, OpState};
use parking_lot::RwLock;
use schemajs_query::errors::QueryError;
use schemajs_query::ops::query_ops::QueryOps;
use schemajs_query::row::Row;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[op2(async)]
#[serde]
pub async fn op_engine_search_rows(
    state: Rc<RefCell<OpState>>,
    #[string] db_name: String,
    #[string] table_name: String,
    #[serde] args: QueryOps,
) -> Result<Vec<Value>, QueryError> {
    let mut mut_state = state.borrow_mut();
    let state = mut_state
        .borrow_mut::<Arc<RwLock<SchemeJsEngine>>>()
        .clone();

    let query_manager = {
        let read_engine = state.read();
        let db = read_engine.find_by_name_ref(db_name.as_str()).unwrap();
        db.query_manager.clone()
    };

    let table = query_manager.get_table(&table_name);
    if let Some(_) = table {
        let s = query_manager
            .search_manager
            .search(&table_name, &args)
            .map_err(|_| QueryError::InvalidQuerySearch(table_name.clone()))?;
        let vals: Vec<Value> = s.iter().filter_map(|row| row.to_json().ok()).collect();
        return Ok(vals);
    }

    Err(QueryError::InvalidQuerySearch(table_name))
}
