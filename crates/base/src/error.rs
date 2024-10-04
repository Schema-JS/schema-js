use serde::{Deserialize, Serialize};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum SjsRtError {
    #[error("Runtime could not be created")]
    UnexpectedRuntimeCreation,

    #[error("Runtime is currently being used")]
    BusyRuntime,
}
