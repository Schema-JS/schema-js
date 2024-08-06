pub mod borsh;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RowSerializationError {
    #[error("Row Serialization error: {0}")]
    SerializationError(String),
    #[error("Row Deserialization error: {0}")]
    DeserializationError(String),
}

pub trait RowSerializer: std::fmt::Debug + Send + Sync + 'static {
    fn serialize(&self, value: &Value) -> Result<Vec<u8>, RowSerializationError>;
    fn deserialize(&self, data: &[u8]) -> Result<Value, RowSerializationError>;
}
