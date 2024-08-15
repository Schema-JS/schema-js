use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize, EnumAsInner)]
pub enum QueryError {
    #[error("Unknown table '{0}'")]
    InvalidTable(String),

    #[error("Primary column '{0}' is not present in table")]
    UnknownPrimaryColumn(String),

    #[error("Required Value not present '{0}'")]
    ValueNotPresent(String),

    #[error("Cannot Serialize")]
    InvalidSerialization,

    #[error("Uid not present")]
    UnknownUid,

    #[error("Invalid Insertion")]
    InvalidInsertion,
}
