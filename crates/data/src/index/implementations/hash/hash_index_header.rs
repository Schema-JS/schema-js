use crate::data_handler::DataHandler;
use crate::data_shard_header::{DataShardHeader, DEFAULT_MAX_OFFSETS};
use crate::U64_SIZE;
use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub const HASH_INDEX_KEY_SIZE: usize = 64;
pub const HASH_INDEX_VALUE_SIZE: usize = 8;
pub const HASH_INDEX_TOTAL_ENTRY_SIZE: usize = HASH_INDEX_KEY_SIZE + HASH_INDEX_VALUE_SIZE;
