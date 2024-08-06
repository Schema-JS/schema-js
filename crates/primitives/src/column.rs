use crate::types::DataTypes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub data_type: DataTypes,
    pub default_value: Option<String>,
    pub required: bool,
    pub comment: Option<String>,
}

impl Column {
    pub fn new(name: &str, data_type: DataTypes) -> Self {
        Self {
            name: String::from(name),
            data_type,
            default_value: None,
            comment: None,
            required: false,
        }
    }

    pub fn set_default_value(mut self, default_value: &str) -> Self {
        self.default_value = Some(default_value.to_string());
        self
    }

    pub fn set_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}
