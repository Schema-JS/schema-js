use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, EnumAsInner, Serialize, Deserialize, Error)]
pub enum ShardErrors {
    #[error("No more positions available")]
    OutOfPositions,
    #[error("Offset is not in file")]
    UnknownOffset,
    #[error("Could not flush")]
    FlushingError,
    #[error("Unknown byte range")]
    ErrorReadingByteRange,
    #[error("Invalid Header")]
    ErrorAddingHeaderOffset,
    #[error("Unknown entry")]
    UnknownEntry,
    #[error("Error adding entry")]
    ErrorAddingEntry,
    #[error("Unknown breaking point")]
    UnknownBreakingPoint,
    #[error("Out of range")]
    OutOfRange,
    #[error("Shard does not exist")]
    UnknownShard,
    #[error("Invalid locking detected")]
    InvalidLocking,
}
