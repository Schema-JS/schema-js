use crate::runtime::SchemeJsRuntime;
use schemajs_engine::engine::SchemeJsEngine;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct RuntimePool {
    pub scheme_js_config: PathBuf,
    pub data_folder: Option<PathBuf>,
    pub engine: Arc<RwLock<SchemeJsEngine>>,
    pub max_capacity: u64,
    pub min_capacity: u64,
    pub workers: RwLock<Vec<SchemeJsRuntime>>,
}

impl RuntimePool {
    pub fn new(
        scheme_js_config: PathBuf,
        data_folder: Option<PathBuf>,
        max_capacity: Option<u64>,
        min_capacity: Option<u64>,
    ) -> Self {
        let engine = SchemeJsEngine::new(data_folder.clone());
        let capacity = max_capacity.unwrap_or(200);
        let min_capacity = min_capacity.unwrap_or(10);

        Self {
            scheme_js_config,
            data_folder,
            engine: Arc::new(RwLock::new(engine)),
            max_capacity: capacity,
            workers: RwLock::new(Vec::with_capacity(capacity as usize)),
            min_capacity,
        }
    }
}
