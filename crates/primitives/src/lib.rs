pub mod collection;
pub mod column;
pub mod database;
pub mod table;
pub mod types;

deno_core::extension!(
    sjs_primitives,
    esm = [
        "src/js/column.ts",
        "src/js/dataTypes.ts",
        "src/js/table.ts",
        "src/js/index.ts"
    ]
);
