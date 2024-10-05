use crate::row::Row;
use crate::serializer::RowSerializationError;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use schemajs_primitives::table::Table;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// `RowData` represents the core structure for storing a row's data in a JSON format.
/// It contains the name of the table and a generic JSON `Value` that stores the actual data.
///
/// # Fields:
/// - `table`: A `String` representing the name of the table to which the row belongs.
/// - `value`: A `serde_json::Value` that holds the data of the row in a flexible, serialized JSON format.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RowData {
    pub value: HashMap<String, DataValue>,
}

/// `RowJson` is a wrapper struct around `RowData`, making it easier to handle rows that
/// store their values in a JSON format. It simplifies managing rows where the data is serialized as JSON.
///
/// # Fields:
/// - `value`: A `RowData` instance that encapsulates both the table name and the actual row data.
#[derive(Clone, Debug)]
pub struct RowJson {
    pub table: Arc<Table>,
    pub values: RowData,
}

impl Row for RowJson {
    type RowData = RowData;

    fn to_data(&self) -> Self::RowData {
        self.values.clone()
    }

    fn to_vec(&self) -> Result<Vec<u8>, RowSerializationError> {
        serde_json::to_vec(&self.values).map_err(|_| {
            RowSerializationError::SerializationError("Row could not be serialized".to_string())
        })
    }

    fn to_map(&self) -> Result<HashMap<String, DataValue>, RowSerializationError> {
        Ok(self.values.value.clone())
    }

    fn from_slice(slice: &[u8], table: Arc<Table>) -> Self {
        RowJson {
            table,
            values: serde_json::from_slice(slice).unwrap(),
        }
    }

    fn from_data(data: Self::RowData, table: Arc<Table>) -> Self {
        RowJson {
            table,
            values: data,
        }
    }

    fn from_map(table: Arc<Table>, data: HashMap<String, DataValue>) -> Result<Self, ()> {
        Ok(RowJson {
            table,
            values: RowData { value: data },
        })
    }

    fn get_table(&self) -> Arc<Table> {
        self.table.clone()
    }

    fn get_value(&self, column: &Column) -> Option<DataValue> {
        let potential_val = self.values.value.get(&column.name);
        match potential_val {
            None => return None,
            Some(val) => Some(val.clone()),
        }
    }

    fn set_value(&mut self, column: &Column, value: DataValue) {
        self.values
            .value
            .entry(column.name.clone())
            .and_modify(|e| *e = value.clone())
            .or_insert(value);
    }

    fn get_table_name(&self) -> String {
        self.table.name.clone()
    }

    fn validate(&self) -> bool {
        todo!()
    }
}
