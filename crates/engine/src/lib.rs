use crate::ops::insert::op_engine_insert_row;

pub mod engine;
pub mod engine_db;
pub mod engine_table;
mod ops;
mod query_error;
pub mod validation_error;

pub mod serializer;
pub mod utils;

deno_core::extension!(
    sjs_engine,
    ops = [op_engine_insert_row],
    esm = ["src/js/ops.ts",]
);
