use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, EnumAsInner, Serialize, Deserialize)]
pub enum TempOffsetTypes {
    WALBased,
    Custom(Option<u64>),
}

impl TempOffsetTypes {
    pub fn get_real_offset(&self) -> Option<u64> {
        match self {
            TempOffsetTypes::WALBased => Some(1),
            TempOffsetTypes::Custom(val) => *val,
        }
    }
}
