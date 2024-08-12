use crate::shard::shards::kv::shard_header::KvShardHeader;

pub fn get_element_offset(index: usize, value_size: usize) -> usize {
    let index_header_size = KvShardHeader::header_size();
    index_header_size + (index * value_size)
}
