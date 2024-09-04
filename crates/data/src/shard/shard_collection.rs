use crate::shard::map_shard::MapShard;
use crate::shard::temp_map_shard::TempMapShard;
use crate::shard::{Shard, ShardConfig, TempShardConfig};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct ShardCollection<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>> {
    pub data: Arc<RwLock<MapShard<S, Opts>>>,
    pub temps: Arc<RwLock<TempMapShard<S, Opts, TempOpts>>>,
}

impl<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>>
    ShardCollection<S, Opts, TempOpts>
{
    pub fn new(folder: PathBuf, prefix: &str, config: Opts, temp_config: TempOpts) -> Self {
        let ctx = MapShard::<S, Opts>::new(folder.clone(), prefix, config);

        let arc_lock = Arc::new(RwLock::new(ctx));

        Self {
            data: arc_lock.clone(),
            temps: Arc::new(RwLock::new(TempMapShard::new(
                folder,
                prefix,
                arc_lock,
                temp_config,
            ))),
        }
    }
}
