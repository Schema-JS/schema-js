use crate::column::Column;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum DataTypes {
    Null,
    Uuid,
    String,
    Boolean,
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum DataValue {
    Null,
    Uuid(Uuid),
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
            DataValue::Uuid(_) => DataTypes::Uuid,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DataValue::Null => String::from("0"),
            DataValue::String(s) => String::from(s),
            DataValue::Boolean(b) => b.to_string(),
            DataValue::Number(n) => n.to_string().to_string(),
            DataValue::Uuid(val) => val.to_string(),
        }
    }
}

impl From<(&Column, &Value)> for DataValue {
    fn from(value: (&Column, &Value)) -> Self {
        match value.0.data_type {
            DataTypes::Null => DataValue::Null,
            DataTypes::Uuid => {
                let str_val = value.1.as_str().unwrap();
                let new_uuid = Uuid::from_str(str_val);
                let x = new_uuid.unwrap();
                DataValue::Uuid(x)
            }
            DataTypes::String => DataValue::String(value.1.as_str().unwrap().to_string()),
            DataTypes::Boolean => DataValue::Boolean(value.1.as_bool().unwrap()),
            DataTypes::Number => DataValue::Number(value.1.as_number().unwrap().clone()),
        }
    }
}
