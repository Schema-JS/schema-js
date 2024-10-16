pub mod errors;
pub mod query_state;

use crate::query_state::ReplQueryState;
use deno_core::{anyhow, located_script_name, serde_v8, v8, JsRuntime, ModuleCodeString};
use schemajs_core::GlobalContext;

deno_core::extension!(sjs_repl, esm = ["src/js/repl.ts",]);

pub async fn run_repl_script(
    runtime: &mut JsRuntime,
    script: String,
) -> anyhow::Result<Option<serde_json::Value>> {
    let res = runtime.execute_script(located_script_name!(), ModuleCodeString::from(script));
    match res {
        Ok(res) => {
            let resolve = runtime.resolve_value(res).await?;
            let scope = &mut runtime.handle_scope();
            let local = v8::Local::new(scope, resolve);
            let to_json = serde_v8::from_v8::<serde_json::Value>(scope, local).ok();
            Ok(to_json)
        }
        Err(e) => Err(e),
    }
}

pub fn get_current_db_context(runtime: &mut JsRuntime) -> GlobalContext {
    let scope = &mut runtime.handle_scope();
    let context = scope.get_current_context();
    let inner_scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(inner_scope);

    let sjs_context_key = serde_v8::to_v8(inner_scope, "SJS_CONTEXT").unwrap();
    let val = global.get(inner_scope, sjs_context_key).unwrap();
    let ctx = serde_v8::from_v8::<GlobalContext>(inner_scope, val).unwrap();

    ctx
}

pub fn get_query_state(ctx: &GlobalContext) -> ReplQueryState {
    match (&ctx.database_name, &ctx.table_name) {
        (Some(db_name), Some(tbl_name)) => ReplQueryState::Table(db_name.clone(), tbl_name.clone()),
        (Some(db_name), None) => ReplQueryState::Database(db_name.clone()),
        (None, None) => ReplQueryState::Global,
        _ => ReplQueryState::Global, // Optional: handle the edge case where only table_name is set
    }
}
