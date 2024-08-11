use crate::index::data::index_data_unit::IndexDataUnit;
use crate::index::data::index_shard_header::IndexShardHeader;

pub fn get_entry_size(key_size: usize, value_size: usize) -> usize {
    let entry_data_size = {
        // The data of an entry is made of 2 IndexDataUnit (Key, Value)
        // + The value of the key and the size.
        let key_size = IndexDataUnit::header_size() + key_size;
        let value_size = IndexDataUnit::header_size() + value_size;
        key_size + value_size
    };
    IndexDataUnit::header_size() + entry_data_size
}

pub fn get_element_offset(index: usize, key_size: usize, value_size: usize) -> usize {
    let index_header_size = IndexShardHeader::header_size();
    index_header_size + (index * (get_entry_size(key_size, value_size)))
}
