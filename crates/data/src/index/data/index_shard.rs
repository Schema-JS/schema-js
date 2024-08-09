use crate::map_shard::MapShard;
use std::path::Path;

pub struct IndexShard {
    pub shard: MapShard,
}

impl IndexShard {
    pub fn new<P: AsRef<Path> + Clone>(
        shards_folder: P,
        shard_prefix: &str,
        max_offsets: Option<u64>,
    ) -> Self {
        Self {
            shard: MapShard::new(shards_folder, shard_prefix, max_offsets),
        }
    }
}
