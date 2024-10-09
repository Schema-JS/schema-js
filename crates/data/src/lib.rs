pub mod data_handler;
pub mod errors;
pub mod fdm;
pub mod shard;
pub mod temp_offset_types;
pub mod utils;

// https://doc.rust-lang.org/std/mem/fn.size_of.html
pub const U64_SIZE: usize = size_of::<u64>();
pub const I64_SIZE: usize = size_of::<i64>();
