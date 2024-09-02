use crate::primitives::Row;
use crate::serializer;
use crate::serializer::RowSerializationError;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

#[derive(Serialize, Deserialize, Clone)]
pub struct RowData {
    pub table: String,
    pub value: serde_json::Value,
}

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

impl From<Vec<u8>> for RowJson {
    fn from(value: Vec<u8>) -> Self {
        RowJson::_deserialize(&value).unwrap()
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
