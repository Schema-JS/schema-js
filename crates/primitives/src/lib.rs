pub mod collection;
pub mod column;
pub mod database;
pub mod table;
pub mod types;

deno_core::extension!(
    sjs_primitives,
    esm_entry_point = "ext:sjs_primitives/src/js/index.ts",
    esm = ["src/js/index.ts"]
);
