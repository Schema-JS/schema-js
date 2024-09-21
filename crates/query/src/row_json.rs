use crate::row::Row;
use crate::serializer;
use crate::serializer::RowSerializationError;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// `RowData` represents the core structure for storing a row's data in a JSON format.
/// It contains the name of the table and a generic JSON `Value` that stores the actual data.
///
/// # Fields:
/// - `table`: A `String` representing the name of the table to which the row belongs.
/// - `value`: A `serde_json::Value` that holds the data of the row in a flexible, serialized JSON format.
#[derive(Serialize, Deserialize, Clone)]
pub struct RowData {
    pub table: String,
    pub value: serde_json::Value,
}

/// `RowJson` is a wrapper struct around `RowData`, making it easier to handle rows that
/// store their values in a JSON format. It simplifies managing rows where the data is serialized as JSON.
///
/// # Fields:
/// - `value`: A `RowData` instance that encapsulates both the table name and the actual row data.
#[derive(Serialize, Deserialize, Clone)]
pub struct RowJson {
    pub value: RowData,
}

impl RowJson {
    fn _serialize(value: &RowData) -> Result<Vec<u8>, RowSerializationError> {
        serde_json::to_vec(value).map_err(|e| {
            RowSerializationError::SerializationError("Error serializing row".to_string())
        })
    }

    fn _deserialize(data: &[u8]) -> Result<RowJson, RowSerializationError> {
        let data = serde_json::from_slice::<RowData>(data).map_err(|e| {
            RowSerializationError::DeserializationError("Error Deserializing row".to_string())
        })?;

        Ok(Self { value: data })
    }
}

impl From<RowData> for RowJson {
    fn from(value: RowData) -> Self {
        RowJson { value }
    }
}

impl serializer::RowSerializer<RowJson> for RowJson {
    fn serialize(&self) -> Result<Vec<u8>, RowSerializationError> {
        Self::_serialize(&self.value)
    }

    fn deserialize(&self, data: &[u8]) -> Result<RowJson, RowSerializationError> {
        Self::_deserialize(data)
    }
}

impl Debug for RowJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl From<&[u8]> for RowJson {
    fn from(value: &[u8]) -> Self {
        RowJson::_deserialize(value).unwrap()
    }
}

impl Row<RowJson> for RowJson {
    fn get_value(&self, column: &Column) -> Option<DataValue> {
        let potential_val = self.value.value.get(column.name.to_string());
        match potential_val {
            None => return None,
            Some(val) => Some(DataValue::from((column, val))),
        }
    }

    fn get_table_name(&self) -> String {
        self.value.table.clone()
    }

    fn validate(&self) -> bool {
        todo!()
    }
}
