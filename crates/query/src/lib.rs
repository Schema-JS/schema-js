use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod errors;
pub mod managers;
pub mod ops;
pub mod row;
pub mod row_json;
mod search;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum RowSerializationError {
    #[error("Row Serialization error: {0}")]
    SerializationError(String),
    #[error("Row Deserialization error: {0}")]
    DeserializationError(String),
}
