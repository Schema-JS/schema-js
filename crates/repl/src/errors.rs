use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, EnumAsInner)]
pub enum ReplError {
    AlreadyInContext,
    UnexpectedUseArgsLength,
    AlreadyInGlobal,
}

#[derive(Serialize, Deserialize)]
pub struct ReplErrorResponse {
    #[serde(rename = "REPL_ERR")]
    pub error: ReplError,
}

impl From<&Value> for ReplErrorResponse {
    fn from(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap()
    }
}
