use crate::implementations::hash::hash_index::HashIndex;
use crate::types::{Index, IndexKey};
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

#[derive(Debug, EnumAsInner, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    Hash,
}

#[derive(Debug)]
pub enum IndexTypeValue {
    Hash(HashIndex),
}

impl IndexTypeValue {
    pub fn as_index(&self) -> Box<&dyn Index> {
        match self {
            IndexTypeValue::Hash(indx) => Box::new(indx),
        }
    }
}
