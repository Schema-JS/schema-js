use crate::manager::SchemeJsManager;
use parking_lot::RwLock;
use schemajs_config::SchemeJsConfig;
use schemajs_data::fdm::FileDescriptorManager;
use schemajs_engine::engine::SchemeJsEngine;
use schemajs_helpers::helper::HelperCall;
use schemajs_internal::manager::InternalManager;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct SjsContext {
    pub config_file: PathBuf,
    pub data_path_folder: Option<PathBuf>,
    pub current_folder: PathBuf,
    pub engine: Arc<RwLock<SchemeJsEngine>>,
    pub internal_manager: Arc<InternalManager>,
    pub task_manager: Arc<RwLock<SchemeJsManager>>,
    pub config: Arc<SchemeJsConfig>,
    pub initialized: AtomicBool,
    pub fdm: Arc<FileDescriptorManager>,
    repl: AtomicBool,
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
            (base_path.clone(), base_path.join("SchemaJS.toml"))
        } else {
            let folder_path = base_path.parent().map_or_else(
                || std::env::current_dir(),
                |parent| Ok(parent.to_path_buf()),
            )?;
            (folder_path.clone(), base_path)
        };

        let config = Arc::new(SchemeJsConfig::new(config_file.clone())?);
        let file_descriptor_manager = Arc::new(FileDescriptorManager::new(
            config.process.max_file_descriptors_in_cache,
        ));

        let data_path = if cfg!(test) {
            let data_folder = folder_path
                .clone()
                .join(".data")
                .join(Uuid::new_v4().to_string());
            if !data_folder.exists() {
                println!("Using test path");
                let _ = std::fs::create_dir_all(&data_folder);
            }
            Some(data_path.unwrap_or(data_folder))
        } else {
            data_path
        };

        let mut engine = Arc::new(RwLock::new(SchemeJsEngine::new(
            data_path.clone(),
            config.clone(),
            helper_tx,
            file_descriptor_manager.clone(),
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
            fdm: file_descriptor_manager,
            repl: AtomicBool::new(true),
        })
    }

    pub fn mark_loaded(&self) {
        self.initialized.store(true, Ordering::SeqCst);
    }

    // Function to check if the struct is loaded
    pub fn is_loaded(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    pub fn mark_repl(&self) {
        self.repl.store(true, Ordering::SeqCst);
    }

    // Function to check if the struct is loaded
    pub fn is_repl(&self) -> bool {
        self.repl.load(Ordering::SeqCst)
    }
}
