use crate::row::{Row, RowBuilder};
use crate::serializer;
use crate::serializer::RowSerializationError;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use schemajs_primitives::table::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// `RowData` represents the core structure for storing a row's data in a JSON format.
/// It contains the name of the table and a generic JSON `Value` that stores the actual data.
///
/// # Fields:
/// - `table`: A `String` representing the name of the table to which the row belongs.
/// - `value`: A `serde_json::Value` that holds the data of the row in a flexible, serialized JSON format.
#[derive(Serialize, Deserialize, Clone)]
pub struct RowData {
    pub value: Value,
}

/// `RowJson` is a wrapper struct around `RowData`, making it easier to handle rows that
/// store their values in a JSON format. It simplifies managing rows where the data is serialized as JSON.
///
/// # Fields:
/// - `value`: A `RowData` instance that encapsulates both the table name and the actual row data.
#[derive(Serialize, Deserialize, Clone)]
pub struct RowJson {
    pub table: Arc<Table>,
    pub value: RowData,
}

impl RowJson {
    fn _serialize(value: &RowData) -> Result<Vec<u8>, RowSerializationError> {
        serde_json::to_vec(value).map_err(|e| {
            RowSerializationError::SerializationError("Error serializing row".to_string())
        })
    }

    fn _deserialize(table: Arc<Table>, data: &[u8]) -> Result<RowJson, RowSerializationError> {
        let data = serde_json::from_slice::<RowData>(data).map_err(|e| {
            RowSerializationError::DeserializationError("Error Deserializing row".to_string())
        })?;

        Ok(Self { value: data, table })
    }
}

impl serializer::RowSerializer<RowJson> for RowJson {
    fn serialize(&self) -> Result<Vec<u8>, RowSerializationError> {
        Self::_serialize(&self.value)
    }

    fn deserialize(
        &self,
        table: Arc<Table>,
        data: &[u8],
    ) -> Result<RowJson, RowSerializationError> {
        Self::_deserialize(table, data)
    }
}

impl Debug for RowJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl RowBuilder<RowJson> for RowJson {
    fn from_slice(table: Arc<Table>, slice: &[u8]) -> RowJson {
        Self::_deserialize(table, slice).unwrap()
    }

    fn from_serializable<R>(table: Arc<Table>, data: R) -> Result<Self, ()>
    where
        R: Serialize,
    {
        let json = serde_json::to_value(data).map_err(|e| ())?;
        Ok(RowJson {
            table,
            value: RowData { value: json },
        })
    }

    fn from_map(table: Arc<Table>, data: HashMap<String, DataValue>) -> Result<RowJson, ()> {
        let value = Value::Object(data.into_iter().map(|e| (e.0, e.1.to_value())).collect());
        Ok(RowJson {
            table,
            value: RowData { value },
        })
    }
}

impl Row<RowJson> for RowJson {
    fn get_table(&self) -> Arc<Table> {
        self.table.clone()
    }

    fn to_map(&self) -> Result<HashMap<String, DataValue>, RowSerializationError> {
        // Ensure that `self.value.value` is an object before proceeding
        let object = self.value.value.as_object().ok_or_else(|| {
            RowSerializationError::DeserializationError("Value in row is unknown".to_string())
        })?;

        // Use `try_collect` pattern to handle potential errors within the mapping process
        let table = self.get_table();
        let res: HashMap<String, DataValue> = object
            .iter()
            .map(|(col_name, value)| {
                // Get the column from the table, return an error if not found
                let col = table.get_column(col_name).ok_or_else(|| {
                    RowSerializationError::DeserializationError(format!(
                        "Column is unknown {}",
                        col_name
                    ))
                })?;

                // Convert the column and value into a DataValue
                Ok((col_name.clone(), DataValue::from((col, value))))
            })
            .collect::<Result<HashMap<_, _>, RowSerializationError>>()?;

        Ok(res)
    }

    fn get_value(&self, column: &Column) -> Option<DataValue> {
        let potential_val = self.value.value.get(column.name.to_string());
        match potential_val {
            None => return None,
            Some(val) => Some(DataValue::from((column, val))),
        }
    }

    // TODO: Use DataValue instead of `Value`
    fn set_value(&mut self, column: &Column, value: DataValue) {
        // Check if the Value is an object
        if let serde_json::Value::Object(ref mut obj) = self.value.value {
            // Insert a new field
            obj.insert(column.name.to_string(), value.to_value());
        }
    }

    fn get_table_name(&self) -> String {
        self.table.name.clone()
    }

    fn validate(&self) -> bool {
        todo!()
    }
}
