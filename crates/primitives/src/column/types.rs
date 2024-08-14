use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum DataTypes {
    Null,
    String,
    Boolean,
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum DataValue {
    Null,
    String(String),
    Boolean(bool),
    Number(serde_json::Number),
}

impl DataValue {
    fn get_type(&self) -> DataTypes {
        match self {
            DataValue::Null => DataTypes::Null,
            DataValue::String(_) => DataTypes::String,
            DataValue::Boolean(_) => DataTypes::Boolean,
            DataValue::Number(_) => DataTypes::Number,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DataValue::Null => String::from("0"),
            DataValue::String(s) => String::from(s),
            DataValue::Boolean(b) => b.to_string(),
            DataValue::Number(n) => n.to_string().to_string(),
        }
    }
}
