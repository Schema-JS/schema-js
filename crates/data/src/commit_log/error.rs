use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, EnumAsInner, Serialize, Deserialize, Error, PartialEq)]
pub enum CommitLogError {
    #[error("Broken Record")]
    BrokenRecord,
    #[error("Unkown Start Delimiter")]
    UnknownStartDelimiter,
    #[error("Out of Memory")]
    OutOfMemory,
    #[error("Can't write in the current range")]
    InvalidRange,
    #[error("Failed Flushing")]
    FailedFlushing,
    #[error("EOF")]
    Eof,
    #[error("Commit Log is locked")]
    LogLocked,
    #[error("No more locks available in collection")]
    MaxLogsReached,
    #[error("Broken Record Known Valid Point (KVP)")]
    BrokenRecordKvp(usize),
}
