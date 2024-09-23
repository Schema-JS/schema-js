use crate::composite_key::CompositeKey;
use crate::data::index_data_unit::IndexDataUnit;
use crate::index_keys::IndexKeyType;
use std::fmt::Debug;

pub trait IndexKey:
    From<Vec<u8>> + Into<Vec<u8>> + Ord + Clone + Into<String> + From<IndexDataUnit>
{
}

pub trait IndexValue: From<Vec<u8>> + Into<Vec<u8>> + Clone + From<IndexDataUnit> {}

pub trait Index: Debug {
    fn to_key(&self, key: CompositeKey) -> IndexKeyType;

    fn bulk_insert(&self, data: Vec<(IndexKeyType, u64)>);

    fn insert(&self, key: IndexKeyType, row_position: u64);

    fn get(&self, key: &IndexKeyType) -> Option<u64>;

    fn remove(&mut self, key: &IndexKeyType) -> Option<u64>;

    fn supported_search_operators(&self) -> Vec<String>;
}
