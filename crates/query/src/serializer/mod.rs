pub mod borsh;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `RowSerializationError` is an enum that defines two types of errors that can occur
/// during the serialization or deserialization of rows:
/// - `SerializationError`: An error that occurs during the serialization process, represented as a string message.
/// - `DeserializationError`: An error that occurs during the deserialization process, also represented as a string message.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RowSerializationError {
    #[error("Row Serialization error: {0}")]
    SerializationError(String),
    #[error("Row Deserialization error: {0}")]
    DeserializationError(String),
}

/// `RowSerializer` is a trait that must be implemented by any type that supports
/// serialization and deserialization of row data. The generic type `V` represents
/// the type that will be deserialized from a serialized byte slice.
///
/// The trait requires two methods:
/// - `serialize`: Converts the row into a `Vec<u8>` for storage or transmission.
///   If serialization fails, a `RowSerializationError` is returned.
/// - `deserialize`: Converts a byte slice into a value of type `V`.
///   If deserialization fails, a `RowSerializationError` is returned.
pub trait RowSerializer<V>: std::fmt::Debug + Send + Sync + 'static {
    /// Serializes the row into a `Vec<u8>`.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)` if serialization succeeds.
    /// - `Err(RowSerializationError)` if serialization fails.
    fn serialize(&self) -> Result<Vec<u8>, RowSerializationError>;

    /// Deserializes a byte slice into a value of type `V`.
    ///
    /// # Parameters
    /// - `data`: The byte slice to be deserialized.
    ///
    /// # Returns
    /// - `Ok(V)` if deserialization succeeds.
    /// - `Err(RowSerializationError)` if deserialization fails.
    fn deserialize(&self, data: &[u8]) -> Result<V, RowSerializationError>;
}
