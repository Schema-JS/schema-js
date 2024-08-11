use crate::index::data::index_data_unit::IndexDataUnit;
use crate::index::types::IndexValue;

#[derive(Debug)]
pub struct RawIndexValue(pub Vec<u8>);

impl From<Vec<u8>> for RawIndexValue {
    fn from(value: Vec<u8>) -> Self {
        RawIndexValue(value)
    }
}

impl Into<Vec<u8>> for RawIndexValue {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl Clone for RawIndexValue {
    fn clone(&self) -> Self {
        RawIndexValue(self.0.clone())
    }
}

impl From<IndexDataUnit> for RawIndexValue {
    fn from(value: IndexDataUnit) -> Self {
        RawIndexValue(value.data)
    }
}

impl IndexValue for RawIndexValue {}
