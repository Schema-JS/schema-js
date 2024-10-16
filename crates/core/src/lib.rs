use serde::{Deserialize, Serialize};

pub mod transpiler;

deno_core::extension!(
    sjs_core,
    esm_entry_point = "ext:sjs_core/src/js/bootstrap.ts",
    esm = [
        "src/js/fieldUtils.ts",
        "src/js/global.ts",
        "src/js/bootstrap.ts",
    ]
);

#[derive(Serialize, Deserialize, Default)]
pub struct GlobalContext {
    #[serde(rename = "tblName")]
    pub table_name: Option<String>,
    #[serde(rename = "dbName")]
    pub database_name: Option<String>,
    #[serde(rename = "REPL_EXIT")]
    pub repl_exit: bool,
}
