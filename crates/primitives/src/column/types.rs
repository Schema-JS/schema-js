use crate::column::Column;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::cmp::Ordering;
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

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner, Eq)]
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

    pub fn to_value(&self) -> Value {
        match self {
            DataValue::Null => Value::Null,
            DataValue::String(s) => Value::String(s.clone()),
            DataValue::Boolean(b) => Value::Bool(b.clone()),
            DataValue::Number(n) => Value::Number(n.clone()),
            DataValue::Uuid(val) => Value::String(val.to_string()),
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

impl PartialEq for DataValue {
    fn eq(&self, other: &DataValue) -> bool {
        match self {
            DataValue::Null => other.is_null(),
            DataValue::Uuid(uuid) => self.as_uuid().unwrap() == uuid,
            DataValue::String(val) => val == other.as_string().unwrap(),
            DataValue::Boolean(val) => val == other.as_boolean().unwrap(),
            DataValue::Number(n) => n == other.as_number().unwrap(),
        }
    }
}

impl PartialOrd for DataValue {
    fn partial_cmp(&self, other: &DataValue) -> Option<Ordering> {
        match (self, other) {
            // Define ordering between variants
            (DataValue::Null, DataValue::Null) => Some(Ordering::Equal),
            (DataValue::Null, _) => Some(Ordering::Less),
            (_, DataValue::Null) => Some(Ordering::Greater),

            (DataValue::Boolean(lhs), DataValue::Boolean(rhs)) => lhs.partial_cmp(rhs),
            (DataValue::Boolean(_), _) => Some(Ordering::Less),
            (_, DataValue::Boolean(_)) => Some(Ordering::Greater),

            (DataValue::Number(lhs), DataValue::Number(rhs)) => {
                lhs.as_f64().partial_cmp(&rhs.as_f64())
            }
            (DataValue::Number(_), _) => Some(Ordering::Less),
            (_, DataValue::Number(_)) => Some(Ordering::Greater),

            (DataValue::String(lhs), DataValue::String(rhs)) => lhs.partial_cmp(rhs),
            (DataValue::String(_), _) => Some(Ordering::Less),
            (_, DataValue::String(_)) => Some(Ordering::Greater),

            (DataValue::Uuid(lhs), DataValue::Uuid(rhs)) => lhs.partial_cmp(rhs),
        }
    }
}

impl Ord for DataValue {
    fn cmp(&self, other: &DataValue) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

macro_rules! data_value_from {
    ($variant:ident, $type:ty, $converter:expr) => {
        impl From<$type> for DataValue {
            fn from(value: $type) -> Self {
                DataValue::$variant($converter(value))
            }
        }
    };
    ($variant:ident, $type:ty) => {
        impl From<$type> for DataValue {
            fn from(value: $type) -> Self {
                DataValue::$variant(value)
            }
        }
    };
}

// Use the macro to implement From for different types
data_value_from!(String, String);
data_value_from!(String, &str, |v: &str| v.to_string());
data_value_from!(Boolean, bool);
data_value_from!(Number, serde_json::Number);
data_value_from!(Uuid, Uuid);
