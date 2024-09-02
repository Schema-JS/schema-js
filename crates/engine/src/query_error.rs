use crate::validation_error::ValidationError;
use enum_as_inner::EnumAsInner;
use schemajs_query::serializer::RowSerializationError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, EnumAsInner, Error, Serialize, Deserialize)]
pub enum InsertionError {
    #[error("Invalid Row Values")]
    ValidationError(#[from] ValidationError),
    #[error("Row could not be serialized")]
    SerializationError(#[from] RowSerializationError),
    #[error("Insertion Error '{0}'")]
    Generic(String),
}

#[derive(Debug, EnumAsInner, Error, Serialize, Deserialize)]
pub enum QueryError {
    #[error("Insertion error: {0}")]
    InvalidInsertion(#[from] InsertionError),
}
