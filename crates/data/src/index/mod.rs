pub mod composite_key;
pub mod data;
mod errors;
pub mod implementations;
pub mod index_type;
pub mod keys;
pub mod types;
mod utils;
pub mod vals;

use std::fmt::Debug;

pub trait Index<K: Ord + Clone + Debug>: Debug {
    fn insert(&self, key: K, row_position: u64);
    fn get(&self, key: &K) -> Option<u64>;
    fn remove(&mut self, key: &K) -> Option<u64>;
    fn search(&self, key: &K) -> Option<u64>;
}
