use crate::context::context::SjsContext;
use crate::manager::task::Task;
use crate::manager::task_duration::TaskDuration;
use crate::manager::tasks::get_all_internal_tasks;
use crate::manager::SchemeJsManager;
use crate::snapshot;
use anyhow::{bail, Error, Result};
use dashmap::DashMap;
use deno_core::_ops::RustToV8;
use deno_core::error::AnyError;
use deno_core::serde_v8::Value;
use deno_core::url::Url;
use deno_core::v8::{GetPropertyNamesArgsBuilder, Global, KeyCollectionMode, Local};
use deno_core::{
    located_script_name, serde_v8, v8, Extension, JsRuntime, ModuleCodeString, ModuleId,
    ModuleSpecifier, PollEventLoopOptions, RuntimeOptions,
};
use schemajs_config::SchemeJsConfig;
use schemajs_engine::engine::SchemeJsEngine;
use schemajs_helpers::helper::{
    Helper, HelperCall, HelperType, SjsHelpersContainer, SjsTableHelpers,
};
use schemajs_internal::get_internal_tables;
use schemajs_internal::manager::InternalManager;
use schemajs_module_loader::ts_module_loader::TypescriptModuleLoader;
use schemajs_primitives::database::Database;
use schemajs_primitives::table::Table;
use schemajs_workers::context::{MainWorkerRuntimeOpts, WorkerRuntimeOpts};
use serde::{Deserialize, Serialize};
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use walkdir::{DirEntry, WalkDir};

thread_local! {
    // NOTE: Suppose we have met `.await` points while initializing a
    // DenoRuntime. In that case, the current v8 isolate's thread-local state can be
    // corrupted by a task initializing another DenoRuntime, so we must prevent this
    // with a Semaphore.

    static RUNTIME_CREATION_SEM: Arc<Semaphore> = Arc::new(Semaphore::new(1));
}

pub struct SchemeJsRuntime {
    pub js_runtime: JsRuntime,
    pub ctx: Arc<SjsContext>,
    pub table_helpers: Arc<SjsTableHelpers>,
    pub busy: AtomicBool,
}

impl SchemeJsRuntime {
    pub async fn new(context: Arc<SjsContext>) -> Result<Self> {
        let extensions: Vec<Extension> = vec![
            schemajs_primitives::sjs_primitives::init_ops(),
            schemajs_core::sjs_core::init_ops(),
            schemajs_engine::sjs_engine::init_ops(),
            schemajs_helpers::sjs_helpers::init_ops(),
        ];

        let runtime_opts = RuntimeOptions {
            extensions,
            is_main: true,
            shared_array_buffer_store: None,
            compiled_wasm_module_store: None,
            startup_snapshot: snapshot::snapshot(),
            module_loader: Some(Rc::new(TypescriptModuleLoader::default())),
            ..Default::default()
        };

        let mut js_runtime = JsRuntime::new(runtime_opts);

        // Bootstrapping Stage
        {
            let script = format!("globalThis.bootstrap()");
            js_runtime
                .execute_script(located_script_name!(), ModuleCodeString::from(script))
                .expect("Failed to execute bootstrap script");
        }

        let table_helpers = Arc::new(SjsTableHelpers(DashMap::new()));

        {
            Self::load(context.clone(), table_helpers.clone(), &mut js_runtime)
                .await
                .unwrap();
        }

        {
            // Put reference to engine
            let op_state_rc = js_runtime.op_state();
            let mut op_state = op_state_rc.borrow_mut();
            op_state.put::<Arc<RwLock<SchemeJsEngine>>>(context.engine.clone());
        }

        {
            if !context.is_loaded() {
                context.internal_manager.init();
            }
        }

        {
            // TODO: Move from here

            if !context.is_loaded() {
                let tasks = get_all_internal_tasks();
                let mut task_manager = context.task_manager.write().unwrap();

                for task in tasks {
                    task_manager.add_task(task);
                }

                task_manager.start_tasks();
            }
        }

        context.mark_loaded();

        Ok(Self {
            js_runtime,
            ctx: context,
            table_helpers,
            busy: AtomicBool::new(false),
        })
    }

    pub async fn load(
        ctx: Arc<SjsContext>,
        helpers: Arc<SjsTableHelpers>,
        js_runtime: &mut JsRuntime,
    ) -> Result<()> {
        let engine_arc = ctx.engine.clone();
        let mut engine = engine_arc.write().unwrap();

        let conf = ctx.config.clone();
        let current_folder = ctx.current_folder.clone();

        let def_scheme_name = conf.default.clone().unwrap().scheme_name;
        let mut databases = conf.workspace.databases.clone();
        databases.push(def_scheme_name.clone());
        let mut evaluated_paths = HashSet::new();

        for database_path in databases {
            let path = current_folder.join(&database_path);

            if evaluated_paths.contains(&path) {
                continue;
            } else {
                evaluated_paths.insert(path.clone());
            }

            let (scheme_name, table_specifiers) = engine.load_database_schema(&path)?;
            let mut tables = vec![];
            for table_specifier in table_specifiers {
                let (_, _, tbl, tbl_helpers) =
                    Self::load_table(js_runtime, table_specifier).await.unwrap();
                let mut rt_helpers = helpers
                    .0
                    .entry(tbl.name.clone())
                    .or_insert(SjsHelpersContainer::default());
                rt_helpers.0.extend(tbl_helpers);
                tables.push(tbl);
            }

            if !ctx.is_loaded() {
                engine.register_tables(scheme_name.as_str(), tables);
            }
        }

        Ok(())
    }

    async fn load_table(
        js_runtime: &mut JsRuntime,
        specifier: ModuleSpecifier,
    ) -> Result<(ModuleSpecifier, ModuleId, Table, Vec<Arc<Helper>>)> {
        let mod_id = js_runtime.load_side_es_module(&specifier).await?;
        let _ = js_runtime.mod_evaluate(mod_id).await?;

        let mut res = {
            let mod_scope = js_runtime.get_module_namespace(mod_id)?;
            {
                let global = {
                    let scope = &mut js_runtime.handle_scope();
                    let mod_obj = mod_scope.open(scope).to_object(scope).unwrap();
                    let default_function_key = v8::String::new(scope, "default").unwrap();
                    let func_obj = mod_obj.get(scope, default_function_key.into()).unwrap();
                    let func = v8::Local::<v8::Function>::try_from(func_obj)?;
                    let undefined = v8::undefined(scope);

                    /// TODO: Handle this error
                    let mut exc = func.call(scope, undefined.into(), &[]).unwrap(); /*
                                                                                    .ok_or_else(Error::msg("Table could not be read"))?*/

                    let is_promise = exc.is_promise();

                    if is_promise {
                        let promise = v8::Local::<v8::Promise>::try_from(exc).unwrap();
                        match promise.state() {
                            v8::PromiseState::Pending => {}
                            v8::PromiseState::Fulfilled | v8::PromiseState::Rejected => {
                                exc = promise.result(scope);
                            }
                        }
                    }

                    let table = deno_core::serde_v8::from_v8::<Table>(scope, exc)?;

                    (table, v8::Global::new(scope, exc))
                };

                let mut table = global.0;
                let global = global.1;
                let mut helpers: Vec<Arc<Helper>> = vec![];

                {
                    let global = { js_runtime.resolve(global).await? };

                    let scope = &mut js_runtime.handle_scope();
                    let table_obj_local = v8::Local::new(scope, global).to_object(scope);

                    if let Some(state) = table_obj_local {
                        let state_key = v8::String::new(scope, "helpers").unwrap().into();
                        if let Some(queries_obj) = state.get(scope, state_key) {
                            if let Some(obj) = queries_obj.to_object(scope) {
                                let props = obj.get_own_property_names(
                                    scope,
                                    GetPropertyNamesArgsBuilder::new()
                                        .mode(KeyCollectionMode::OwnOnly)
                                        .build(),
                                );
                                let helper_indexes =
                                    serde_v8::from_v8::<Vec<u32>>(scope, props.unwrap().into())?;

                                {
                                    for helper_indx in helper_indexes {
                                        let helper = obj.get_index(scope, helper_indx);
                                        if let Some(helper) = helper {
                                            let helper_val = helper.to_object(scope).unwrap();

                                            let (identifier_key, internal_type_key, cb_key) = (
                                                v8::String::new(scope, "identifier")
                                                    .unwrap()
                                                    .into(),
                                                v8::String::new(scope, "internalType")
                                                    .unwrap()
                                                    .into(),
                                                v8::String::new(scope, "cb").unwrap().into(),
                                            );

                                            let (identifier, internal_type, cb) = (
                                                helper_val.get(scope, identifier_key).unwrap(),
                                                helper_val.get(scope, internal_type_key).unwrap(),
                                                helper_val.get(scope, cb_key).unwrap(),
                                            );

                                            let identifier =
                                                serde_v8::from_v8::<String>(scope, identifier)?;
                                            let internal_type = serde_v8::from_v8::<HelperType>(
                                                scope,
                                                internal_type,
                                            )?;
                                            let cb = v8::Local::<v8::Function>::try_from(cb)?;
                                            let global = v8::Global::new(scope, cb);
                                            helpers.push(Arc::new(Helper {
                                                identifier,
                                                internal_type,
                                                func: global,
                                            }));
                                        }
                                    }
                                };
                            }
                        }
                    }
                }

                (table, helpers)
            }
        };

        let (mut table, helpers) = res;

        table.init();

        table.metadata.set_module_id(mod_id);

        Ok((specifier, mod_id, table, helpers))
    }

    pub async fn call_helper(&mut self, helper_call: HelperCall) {
        match helper_call {
            HelperCall::CustomQuery {
                identifier,
                req,
                table,
                response,
            } => {
                let helper = self
                    .table_helpers
                    .find_custom_query_helper(&table, &identifier);

                self.execute_helper(req, Some(response), helper).await;
            }
            HelperCall::InsertHook { table, rows } => {
                let helper = self
                    .table_helpers
                    .find_hook_helper(&table, HelperType::InsertHook);
                let arr_to_val = serde_json::to_value(rows);
                if let Ok(val) = arr_to_val {
                    self.execute_helper(val, None, helper).await
                }
            }
        }
    }

    async fn execute_helper(
        &mut self,
        req: serde_json::Value,
        response: Option<UnboundedSender<serde_json::Value>>,
        helper: Option<Arc<Helper>>,
    ) {
        if let Some(helper) = helper {
            let req = {
                let scope = &mut self.js_runtime.handle_scope();
                serde_v8::to_v8(scope, req).map(|e| v8::Global::new(scope, e))
            };
            match req {
                Ok(req_val) => {
                    let call = self.js_runtime.call_with_args(&helper.func, &[req_val]);
                    let result = self
                        .js_runtime
                        .with_event_loop_promise(call, PollEventLoopOptions::default())
                        .await;
                    match result {
                        Ok(res) => {
                            let scope = &mut self.js_runtime.handle_scope();
                            let local = v8::Local::new(scope, res);
                            let to_val = serde_v8::from_v8::<serde_json::Value>(scope, local);
                            let to_val = to_val.ok().unwrap_or_else(|| serde_json::Value::Null);
                            if let Some(response) = response {
                                let _ = response.send(to_val);
                            }
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
    }

    // Method to release the lock
    pub(crate) fn release_lock(&self) {
        self.busy.store(false, Ordering::Release);
    }

    pub fn acquire_lock(&self) -> Result<(), ()> {
        match self
            .busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub async fn acquire() -> OwnedSemaphorePermit {
        RUNTIME_CREATION_SEM
            .with(|v| v.clone())
            .acquire_owned()
            .await
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::context::context::SjsContext;
    use crate::manager::task::{Task, TaskCallback};
    use crate::manager::task_duration::TaskDuration;
    use crate::manager::SchemeJsManager;
    use crate::runtime::SchemeJsRuntime;
    use deno_core::{located_script_name, serde_json, v8};
    use schemajs_helpers::create_helper_channel;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_runtime_config_as_folder() -> anyhow::Result<()> {
        let (tx, rx) = create_helper_channel(1);
        let create_rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
            PathBuf::from("./test_cases/default-db"),
            None,
            tx,
        )?))
        .await?;

        assert_eq!(
            create_rt.ctx.current_folder,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db")
        );
        assert_eq!(
            create_rt.ctx.config_file,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db/SchemeJS.toml")
        );

        Ok(())
    }

    #[tokio::test]
    pub async fn test_runtime_insert() -> anyhow::Result<()> {
        let (tx, rx) = create_helper_channel(1);
        let data_path = format!("./test_cases/data/{}", Uuid::new_v4().to_string());
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from(data_path.as_str()));

        std::fs::create_dir_all(data_path.clone()).unwrap();

        let now = std::time::Instant::now();
        {
            let mut create_rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
                PathBuf::from("./test_cases/default-db"),
                Some(data_path.clone()),
                tx,
            )?))
            .await?;

            let num_inserts = 10_000;
            let mut script = String::new();

            for i in 0..num_inserts {
                script.push_str(&format!(
                    r#"globalThis.SchemeJS.insert("{}", "{}", {});"#,
                    "public",
                    "users",
                    serde_json::json!({
                        "id": "ABCD"
                    })
                    .to_string()
                ));
            }
            create_rt
                .js_runtime
                .execute_script(located_script_name!(), script)?;
        }
        let elapsed = now.elapsed();
        println!("Elapsed: {:.5?}", elapsed);

        std::fs::remove_dir_all(data_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    pub async fn test_runtime_insert_file_persistence() -> anyhow::Result<()> {
        let (tx, rx) = create_helper_channel(1);
        let data_path = format!("./test_cases/data/{}", Uuid::new_v4().to_string());
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from(data_path.as_str()));

        std::fs::create_dir_all(data_path.clone()).unwrap();
        let now = std::time::Instant::now();

        for _ in 0..2 {
            {
                let mut create_rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
                    PathBuf::from("./test_cases/default-db"),
                    Some(data_path.clone()),
                    tx.clone(),
                )?))
                .await?;

                let num_inserts = 5001;
                let mut script = String::new();

                for i in 0..num_inserts {
                    script.push_str(&format!(
                        r#"globalThis.SchemeJS.insert("{}", "{}", {});"#,
                        "public",
                        "users",
                        serde_json::json!({
                            "id": "ABCD"
                        })
                        .to_string()
                    ));
                }
                create_rt
                    .js_runtime
                    .execute_script(located_script_name!(), script)?;
            }
        }

        let elapsed = now.elapsed();
        println!("Elapsed: {:.5?}", elapsed);

        let mut last_rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
            PathBuf::from("./test_cases/default-db"),
            Some(data_path.clone()),
            tx,
        )?))
        .await?;

        let val = {
            let engine = last_rt.ctx.engine.clone();
            let reader = engine.read().unwrap();
            let db = reader.find_by_name_ref("public").unwrap();
            let table = db.query_manager.tables.get("users").unwrap();
            let table_read = table.data.read().unwrap();
            let header_reader = table_read.current_master_shard.header.read().unwrap();
            println!("{}", header_reader.get_next_available_index().unwrap());
            println!("{}", header_reader.get_last_offset_index());

            (
                header_reader.get_last_offset_index(),
                header_reader.get_next_available_index().unwrap(),
                table_read.get_element(2000).is_err(),
            )
        };

        std::fs::remove_dir_all(data_path).unwrap();

        assert_eq!(val.0, 1999);
        assert_eq!(val.1, 2000);
        assert!(val.2);

        Ok(())
    }

    #[tokio::test]
    pub async fn test_runtime_insert_with_manager() -> anyhow::Result<()> {
        let (tx, rx) = create_helper_channel(1);
        let data_path = format!("./test_cases/data/{}", Uuid::new_v4().to_string());
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from(data_path.as_str()));
        let now = std::time::Instant::now();

        {
            let mut rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
                PathBuf::from("./test_cases/default-db"),
                Some(data_path.clone()),
                tx,
            )?))
            .await?;

            let num_inserts = 9500;
            let mut script = String::new();
            println!("To be inserted");

            for i in 0..num_inserts {
                script.push_str(&format!(
                    r#"globalThis.SchemeJS.insert("{}", "{}", {});"#,
                    "public",
                    "users",
                    serde_json::json!({
                        "id": "ABCD"
                    })
                    .to_string()
                ));
            }

            println!("Inserted");

            rt.js_runtime
                .execute_script(located_script_name!(), script)?;

            // Example: Stop the reconciler and other tasks after some time
            //tokio::time::sleep(Duration::from_secs(20)).await;
            //manager.stop_tasks();

            println!("Executed");
        }

        std::fs::remove_dir_all(data_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    pub async fn test_runtime_config_as_file() -> anyhow::Result<()> {
        let (tx, rx) = create_helper_channel(1);
        let create_rt = SchemeJsRuntime::new(Arc::new(SjsContext::new(
            PathBuf::from("./test_cases/default-db/CustomSchemeJS.toml"),
            None,
            tx,
        )?))
        .await?;

        assert_eq!(
            create_rt.ctx.current_folder,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db")
        );
        assert_eq!(
            create_rt.ctx.config_file,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db/CustomSchemeJS.toml")
        );

        Ok(())
    }
}

unsafe impl Send for SchemeJsRuntime {}
