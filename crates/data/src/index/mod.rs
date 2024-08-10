pub mod composite_key;
mod data;
mod implementations;
mod index_child;
mod index_type;
mod keys;
mod types;

use std::fmt::Debug;

pub trait Index<K: Ord + Clone + Debug>: Debug {
    fn insert(&mut self, key: K, row_position: u64);
    fn get(&self, key: &K) -> Option<u64>;
    fn remove(&mut self, key: &K) -> Option<u64>;
    fn search(&self, key: &K) -> Option<u64>;
}
