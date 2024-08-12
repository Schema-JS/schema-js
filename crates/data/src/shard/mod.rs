use crate::errors::ShardErrors;
use std::path::PathBuf;
use uuid::Uuid;
pub mod map_shard;
pub mod shard_collection;
pub mod shards;
pub mod temp_map_shard;

pub trait ShardConfig: Clone {}

pub trait Shard<Opts: ShardConfig> {
    fn new(path: PathBuf, opts: Opts, uuid: Option<Uuid>) -> Self;

    fn has_space(&self) -> bool;

    fn get_path(&self) -> PathBuf;

    fn get_last_index(&self) -> i64;

    fn read_item_from_index(&self, index: usize) -> Result<Vec<u8>, ShardErrors>;

    fn insert_item(&self, data: Vec<u8>) -> Result<(), ShardErrors>;

    fn get_id(&self) -> String;
}

pub trait TempShardConfig<Opts: ShardConfig> {
    fn to_config(&self) -> Opts;
}
