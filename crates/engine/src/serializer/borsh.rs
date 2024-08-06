use crate::serializer::{RowSerializationError, RowSerializer};
use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum BorshJsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<BorshJsonValue>),
    Object(HashMap<String, BorshJsonValue>),
}

impl From<&Value> for BorshJsonValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => BorshJsonValue::Null,
            Value::Bool(b) => BorshJsonValue::Bool(*b),
            Value::Number(n) => BorshJsonValue::Number(n.as_f64().unwrap()), // assuming all numbers can be converted to f64
            Value::String(s) => BorshJsonValue::String(s.clone()),
            Value::Array(arr) => {
                BorshJsonValue::Array(arr.iter().map(BorshJsonValue::from).collect())
            }
            Value::Object(obj) => BorshJsonValue::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), BorshJsonValue::from(v)))
                    .collect(),
            ),
        }
    }
}

impl Into<Value> for BorshJsonValue {
    fn into(self) -> Value {
        match self {
            BorshJsonValue::Null => Value::Null,
            BorshJsonValue::Bool(b) => Value::Bool(b),
            BorshJsonValue::Number(n) => Value::Number(serde_json::Number::from_f64(n).unwrap()),
            BorshJsonValue::String(s) => Value::String(s),
            BorshJsonValue::Array(arr) => Value::Array(arr.into_iter().map(Into::into).collect()),
            BorshJsonValue::Object(obj) => Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<serde_json::Map<String, Value>>(),
            ),
        }
    }
}

#[derive(Default, Debug)]
pub struct BorshRowSerializer;

impl RowSerializer for BorshRowSerializer {
    fn serialize(&self, value: &Value) -> Result<Vec<u8>, RowSerializationError> {
        let borsh_value: BorshJsonValue = value.into();
        borsh::to_vec(&borsh_value)
            .map_err(|e| RowSerializationError::SerializationError(e.to_string()))
    }

    fn deserialize(&self, data: &[u8]) -> Result<Value, RowSerializationError> {
        let borsh_value = BorshJsonValue::try_from_slice(data)
            .map_err(|e| RowSerializationError::DeserializationError(e.to_string()))?;
        Ok(borsh_value.into())
    }
}
