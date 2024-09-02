use schemajs_index::index_type::IndexType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Index {
    pub name: String,
    pub members: Vec<String>,
    pub index_type: IndexType,
}
