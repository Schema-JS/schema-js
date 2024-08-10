use crate::index::data::index_data_unit::IndexDataUnit;
use std::cmp::Ordering;

pub trait IndexKey:
    From<Vec<u8>> + Into<Vec<u8>> + Ord + Clone + Into<String> + From<IndexDataUnit>
{
}
