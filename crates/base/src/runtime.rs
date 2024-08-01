use crate::snapshot;
use anyhow::{bail, Error, Result};
use deno_core::_ops::RustToV8;
use deno_core::url::Url;
use deno_core::{
    located_script_name, v8, Extension, JsRuntime, ModuleCodeString, ModuleId, ModuleSpecifier,
    RuntimeOptions,
};
use schemajs_config::SchemeJsConfig;
use schemajs_engine::engine::{ArcSchemeJsEngine, SchemeJsEngine};
use schemajs_module_loader::ts_module_loader::TypescriptModuleLoader;
use schemajs_primitives::database::Database;
use schemajs_primitives::table::Table;
use schemajs_workers::context::{MainWorkerRuntimeOpts, WorkerRuntimeOpts};
use serde::{Deserialize, Serialize};
use std::cell::{RefCell, RefMut};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use walkdir::{DirEntry, WalkDir};

pub struct SchemeJsRuntime {
    pub js_runtime: JsRuntime,
    pub config: WorkerRuntimeOpts,
    pub config_file: PathBuf,
    pub current_folder: PathBuf,
    pub engine: SchemeJsEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerContextInitOpts {
    pub config_path: PathBuf,
}

impl SchemeJsRuntime {
    pub async fn new(opts: WorkerContextInitOpts) -> Result<Self> {
        let WorkerContextInitOpts { config_path } = opts;

        // Determine the base path by joining the current directory with the config path
        let base_path = std::env::current_dir()?.join(&config_path);

        // Determine the appropriate folder path and config file path
        let (folder_path, config_file) = if base_path.is_dir() {
            (base_path.clone(), base_path.join("SchemeJS.toml"))
        } else {
            let folder_path = base_path.parent().map_or_else(
                || std::env::current_dir(),
                |parent| Ok(parent.to_path_buf()),
            )?;
            (folder_path.clone(), base_path)
        };

        let config = SchemeJsConfig::new(config_file.clone())?;

        let extensions: Vec<Extension> = vec![
            schemajs_primitives::sjs_primitives::init_ops(),
            schemajs_core::sjs_core::init_ops(),
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

        Ok(Self {
            js_runtime,
            config: WorkerRuntimeOpts::Main(MainWorkerRuntimeOpts { config }),
            config_file,
            current_folder: folder_path,
            engine: SchemeJsEngine::new(),
        })
    }

    pub async fn load(&mut self) -> Result<()> {
        match &self.config {
            WorkerRuntimeOpts::Main(conf) => {
                let databases = conf.config.workspace.databases.clone();

                for database_path in databases {
                    let path = self.current_folder.join(&database_path);
                    self.load_database_schema(&path).await?;
                }

                Ok(())
            }
        }
    }

    async fn load_database_schema(&mut self, path: &PathBuf) -> Result<()> {
        if !path.exists() {
            bail!(
                "Trying to access a database schema that does not exist: {}",
                path.to_string_lossy()
            );
        }

        let schema_name = path.file_name().unwrap().to_str().unwrap();

        // Create Database structure in Memory
        {
            self.engine.add_database(schema_name);
        }

        let table_path = path.join("tables").canonicalize()?;
        let table_walker = WalkDir::new(table_path).into_iter().filter_map(|e| e.ok());

        let mut loaded_tables: Vec<Table> = vec![];

        for table_file in table_walker {
            if Self::is_js_or_ts(&table_file) {
                let url = ModuleSpecifier::from_file_path(table_file.path()).unwrap();
                let (specifier, id, table) = self.load_table(url).await?;
                loaded_tables.push(table);
            }
        }

        let mut db = self.engine.find_by_name(schema_name.to_string()).unwrap();
        for table in loaded_tables {
            db.add_table(table);
        }

        Ok(())
    }

    async fn load_table(
        &mut self,
        specifier: ModuleSpecifier,
    ) -> Result<(ModuleSpecifier, ModuleId, Table)> {
        let mod_id = self.js_runtime.load_side_es_module(&specifier).await?;
        let _ = self.js_runtime.mod_evaluate(mod_id).await?;

        let mut table = {
            let mod_scope = self.js_runtime.get_module_namespace(mod_id)?;
            let scope = &mut self.js_runtime.handle_scope();
            {
                let mod_obj = mod_scope.open(scope).to_object(scope).unwrap();
                let default_function_key = v8::String::new(scope, "default").unwrap();
                let func_obj = mod_obj.get(scope, default_function_key.into()).unwrap();
                let func = v8::Local::<v8::Function>::try_from(func_obj)?;
                let undefined = v8::undefined(scope);

                /// TODO: Handle this error
                let mut exc = func.call(scope, undefined.into(), &[]).unwrap();                /*
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

                deno_core::serde_v8::from_v8::<Table>(scope, exc)?
            }
        };

        table.set_module_id(mod_id);

        Ok((specifier, mod_id, table))
    }

    fn is_js_or_ts(entry: &DirEntry) -> bool {
        entry
            .path()
            .extension()
            .map_or(false, |ext| ext == "js" || ext == "ts")
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::{SchemeJsRuntime, WorkerContextInitOpts};
    use std::path::PathBuf;

    #[tokio::test]
    pub async fn test_runtime_config_as_folder() -> anyhow::Result<()> {
        let create_rt = SchemeJsRuntime::new(WorkerContextInitOpts {
            config_path: PathBuf::from("./test_cases/default-db"),
        })
        .await?;

        assert_eq!(
            create_rt.current_folder,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db")
        );
        assert_eq!(
            create_rt.config_file,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db/SchemeJS.toml")
        );

        Ok(())
    }

    #[tokio::test]
    pub async fn test_runtime_config_as_file() -> anyhow::Result<()> {
        let create_rt = SchemeJsRuntime::new(WorkerContextInitOpts {
            config_path: PathBuf::from("./test_cases/default-db/CustomSchemeJS.toml"),
        })
        .await?;

        assert_eq!(
            create_rt.current_folder,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db")
        );
        assert_eq!(
            create_rt.config_file,
            std::env::current_dir()
                .unwrap()
                .join("./test_cases/default-db/CustomSchemeJS.toml")
        );

        Ok(())
    }
}
