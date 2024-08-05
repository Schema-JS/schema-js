pub mod data_shard;
mod data_shard_header;
mod errors;
pub mod map_shard;
pub mod temp_map_shard;
pub mod utils;

// https://doc.rust-lang.org/std/mem/fn.size_of.html
pub const U64_SIZE: usize = 8;
