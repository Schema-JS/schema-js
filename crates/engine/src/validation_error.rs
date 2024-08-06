use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, EnumAsInner, Error, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Expected a string for column '{0}'")]
    ExpectedString(String),
    #[error("Expected a boolean for column '{0}'")]
    ExpectedBoolean(String),
    #[error("Expected an integer for column '{0}'")]
    ExpectedInteger(String),
    #[error("Expected a float for column '{0}'")]
    ExpectedFloat(String),
    #[error("Missing column '{0}'")]
    MissingColumn(String),
}
