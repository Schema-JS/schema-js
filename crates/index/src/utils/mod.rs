use crate::data::index_data_unit::IndexDataUnit;

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
