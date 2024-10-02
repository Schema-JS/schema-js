use crate::services::shared::shared::data_value::ValueType;
use schemajs_primitives::column::types::DataValue;
use std::str::FromStr;
use uuid::Uuid;

pub fn convert_to_data_value(val: ValueType) -> DataValue {
    match val {
        ValueType::NullValue(_) => DataValue::Null,
        ValueType::UuidValue(u) => DataValue::Uuid(Uuid::from_str(&u).unwrap_or(Uuid::nil())),
        ValueType::StringValue(s) => DataValue::String(s),
        ValueType::BoolValue(b) => DataValue::Boolean(b),
        ValueType::NumberValue(n) => {
            DataValue::Number(serde_json::value::Number::from_f64(n as f64).unwrap())
        }
    }
}
