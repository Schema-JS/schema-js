use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, EnumAsInner, Serialize, Deserialize, Error)]
pub enum CursorError {
    #[error("Not enough bytes")]
    InvalidRange,
}
