pub mod engine;

use crate::engine::ArcSchemeJsEngine;
use deno_core::op2;
use deno_core::OpState;
use schemajs_primitives::table::Table;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRegisterOpOpts {
    pub table_name: String,
    pub db_name: String,
}

#[op2]
fn op_register_table(state: &mut OpState, #[serde] data: TableRegisterOpOpts) {
    let mut engine = state.borrow_mut::<ArcSchemeJsEngine>();

    let mut engine = engine.borrow_mut();
    let mut db = engine.find_by_name(data.db_name);

    if let Some(database) = db {
        database.add_table(Table {
            name: data.table_name,
            columns: Default::default(),
        })
    }
}

deno_core::extension!(
    sjs_engine,
    ops = [
        op_register_table
    ],
    options = {
        engine: ArcSchemeJsEngine
    },
    state = |state, options| {
        state.put(options.engine);
    }
);
