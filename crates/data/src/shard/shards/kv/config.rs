use crate::shard::ShardConfig;

#[derive(Debug, Clone)]
pub struct KvShardConfig {
    pub value_size: usize,
    pub max_capacity: Option<u64>,
}

impl ShardConfig for KvShardConfig {}
