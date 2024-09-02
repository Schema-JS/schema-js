use crate::data::index_data_unit::IndexDataUnit;
use crate::types::IndexKey;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct StringIndexKey(pub String);

impl From<Vec<u8>> for StringIndexKey {
    fn from(value: Vec<u8>) -> Self {
        StringIndexKey(String::from_utf8(value).unwrap())
    }
}

impl Into<Vec<u8>> for StringIndexKey {
    fn into(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl Ord for StringIndexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Eq for StringIndexKey {}

impl PartialEq<Self> for StringIndexKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd<Self> for StringIndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Clone for StringIndexKey {
    fn clone(&self) -> Self {
        StringIndexKey(self.0.clone())
    }
}

impl Into<String> for StringIndexKey {
    fn into(self) -> String {
        self.0
    }
}

impl From<IndexDataUnit> for StringIndexKey {
    fn from(value: IndexDataUnit) -> Self {
        StringIndexKey(String::from_utf8(value.data).unwrap())
    }
}

impl IndexKey for StringIndexKey {}
