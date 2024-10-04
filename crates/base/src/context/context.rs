use crate::manager::SchemeJsManager;
use schemajs_config::SchemeJsConfig;
use schemajs_engine::engine::SchemeJsEngine;
use schemajs_helpers::helper::HelperCall;
use schemajs_internal::manager::InternalManager;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;

pub struct SjsContext {
    pub config_file: PathBuf,
    pub data_path_folder: Option<PathBuf>,
    pub current_folder: PathBuf,
    pub engine: Arc<RwLock<SchemeJsEngine>>,
    pub internal_manager: Arc<InternalManager>,
    pub task_manager: Arc<RwLock<SchemeJsManager>>,
    pub config: Arc<SchemeJsConfig>,
    pub initialized: AtomicBool,
}

impl SjsContext {
    pub fn new(
        config_path: PathBuf,
        data_path: Option<PathBuf>,
        helper_tx: Sender<HelperCall>,
    ) -> anyhow::Result<Self> {
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

        let config = Arc::new(SchemeJsConfig::new(config_file.clone())?);
        let mut engine = Arc::new(RwLock::new(SchemeJsEngine::new(
            data_path.clone(),
            config.clone(),
            helper_tx,
        )));
        let mut internal_manager = Arc::new(InternalManager::new(engine.clone()));
        let mut manager = Arc::new(RwLock::new(SchemeJsManager::new(engine.clone())));

        Ok(Self {
            config_file,
            data_path_folder: data_path.clone(),
            current_folder: folder_path,
            engine,
            internal_manager,
            task_manager: manager,
            config,
            initialized: AtomicBool::new(false),
        })
    }

    pub fn mark_loaded(&self) {
        self.initialized.store(true, Ordering::SeqCst);
    }

    // Function to check if the struct is loaded
    pub fn is_loaded(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
}
