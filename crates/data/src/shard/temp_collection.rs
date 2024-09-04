use crate::errors::ShardErrors;
use crate::shard::map_shard::MapShard;
use crate::shard::temp_map_shard::TempMapShard;
use crate::shard::{Shard, ShardConfig, TempShardConfig};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct TempCollection<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>> {
    pub target_shard: Arc<RwLock<MapShard<S, Opts>>>,
    pub temps: Arc<Vec<RwLock<TempMapShard<S, Opts, TempOpts>>>>,
    counter: AtomicUsize,
}

impl<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>>
    TempCollection<S, Opts, TempOpts>
{
    pub fn new(
        target_shard: Arc<RwLock<MapShard<S, Opts>>>,
        capacity: u64,
        folder: PathBuf,
        prefix: &str,
        temp_config: TempOpts,
    ) -> Self {
        let mut temps = vec![];

        for _ in 0..capacity {
            temps.push(RwLock::new(TempMapShard::new(
                folder.clone(),
                prefix,
                target_shard.clone(),
                temp_config.clone(),
            )));
        }

        Self {
            target_shard,
            temps: Arc::new(temps),
            counter: AtomicUsize::new(0),
        }
    }

    fn get_next_shard(&self) -> &RwLock<TempMapShard<S, Opts, TempOpts>> {
        let index = self.counter.fetch_add(1, Ordering::Relaxed) % self.temps.len();
        &self.temps[index]
    }

    pub fn reconcile_all(&self) {
        for temp in self.temps.iter() {
            temp.write().unwrap().reconcile_all()
        }
    }

    pub fn insert(&self, data: Vec<u8>) -> Result<u64, ShardErrors> {
        let mut next_shard = self
            .get_next_shard()
            .write()
            .map_err(|_e| ShardErrors::InvalidLocking)?;
        next_shard.insert_row(data)
    }
}
