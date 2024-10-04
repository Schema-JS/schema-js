use crate::ops::insert::op_engine_insert_row;
use deno_core::error::AnyError;
use deno_core::{op2, OpState};

pub mod engine;
pub mod engine_db;
mod ops;
mod query_error;
pub mod utils;
pub mod validation_error;

#[op2(fast)]
pub fn sjs_op_print(state: &mut OpState, #[string] msg: &str) -> Result<(), AnyError> {
    println!("{}", msg);

    Ok(())
}

deno_core::extension!(
    sjs_engine,
    ops = [op_engine_insert_row, sjs_op_print],
    esm = ["src/js/ops.ts",]
);
