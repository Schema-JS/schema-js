use crate::serializer::RowSerializer;
use schemajs_primitives::column::types::DataValue;
use schemajs_primitives::column::Column;
use std::hash::Hash;

pub trait Row<T>: RowSerializer<T> + From<Vec<u8>> {
    fn get_value(&self, column: &Column) -> Option<DataValue>;
    fn get_table_name(&self) -> String;
    fn validate(&self) -> bool;
}
