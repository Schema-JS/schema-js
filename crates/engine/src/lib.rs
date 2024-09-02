use crate::ops::insert::op_engine_insert_row;

pub mod engine;
pub mod engine_db;
mod ops;
mod query_error;
pub mod utils;
pub mod validation_error;

deno_core::extension!(
    sjs_engine,
    ops = [op_engine_insert_row],
    esm = ["src/js/ops.ts",]
);
