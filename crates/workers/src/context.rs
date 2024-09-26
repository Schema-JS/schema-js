use enum_as_inner::EnumAsInner;
use schemajs_config::SchemeJsConfig;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MainWorkerRuntimeOpts {
    pub config: Arc<SchemeJsConfig>,
}

#[derive(Debug, Clone, EnumAsInner)]
pub enum WorkerRuntimeOpts {
    Main(MainWorkerRuntimeOpts),
}
