use crate::shard::map_shard::MapShard;
use crate::shard::shards::data_shard::shard::DataShard;
use crate::shard::{Shard, ShardConfig, TempShardConfig};
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub struct OnReconcileCb {
    func: Option<Box<dyn Fn(&Vec<u8>, usize) -> Result<(), ()>>>,
}

impl Debug for OnReconcileCb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function pointer")
    }
}

pub type OnReconcileFn = Box<dyn Fn(&Vec<u8>, usize) -> Result<(), ()>>;

#[derive(Debug)]
pub struct TempMapShard<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>> {
    folder: PathBuf,
    prefix: String,
    parent_shard: Arc<RwLock<MapShard<S, Opts>>>,
    pub temp_shards: RwLock<Vec<S>>,
    temp_opts: TempOpts,
    on_reconcile: OnReconcileCb,
}

impl<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>>
    TempMapShard<S, Opts, TempOpts>
{
    pub fn new(
        folder: PathBuf,
        prefix: &str,
        parent_shard: Arc<RwLock<MapShard<S, Opts>>>,
        temp_opts: TempOpts,
    ) -> Self {
        TempMapShard {
            parent_shard,
            folder,
            prefix: prefix.to_string(),
            temp_shards: RwLock::new(vec![]),
            temp_opts,
            on_reconcile: OnReconcileCb { func: None },
        }
    }

    pub fn set_on_reconcile(&mut self, data: Box<dyn Fn(&Vec<u8>, usize) -> Result<(), ()>>) {
        self.on_reconcile = OnReconcileCb { func: Some(data) };
    }

    fn create_shard(&self) -> S {
        let shard_path = self.folder.join(format!(
            "{}{}",
            self.prefix.clone(),
            Uuid::new_v4().to_string()
        ));
        S::new(shard_path, self.temp_opts.to_config(), None)
    }

    pub fn insert_row(&self, data: Vec<u8>) {
        let find_usable_shard = {
            let mut shards = self.temp_shards.read().unwrap();
            shards.iter().position(|i| i.has_space())
        };

        let shard_index = match find_usable_shard {
            None => {
                {
                    // Reconcile last
                    self.reconcile_specific(None);
                }
                let mut shards = self.temp_shards.write().unwrap();
                let shard = self.create_shard();
                shards.push(shard);
                shards.len() - 1
            }
            Some(shard) => shard,
        };

        {
            let mut shards = self.temp_shards.read().unwrap();
            shards.get(shard_index).unwrap().insert_item(data).unwrap();
        }
    }

    fn get_reconciliation_data(shard: &S) -> (&S, Range<i64>) {
        let indexes = {
            let last_index = shard.get_last_index();
            if last_index < 0 {
                0..0
            } else {
                0..(last_index + 1)
            }
        };

        (shard, indexes)
    }

    // Maybe async?
    fn call_on_reconcile(&self, data: &Vec<u8>, pos: usize) -> Result<(), ()> {
        match &self.on_reconcile.func {
            None => Ok(()),
            Some(cb) => cb(data, pos),
        }
    }

    fn reconcile(&self, from: &S, target: &mut MapShard<S, Opts>) {
        let (shard, indexes) = Self::get_reconciliation_data(from);
        for item_index in indexes {
            let binary_item = shard.read_item_from_index(item_index as usize).unwrap();
            let pos = target.insert_row(binary_item.clone());
            self.call_on_reconcile(&binary_item, pos).unwrap();
        }
    }

    pub fn reconcile_all(&self) {
        let mut parent_writer = self.parent_shard.write().unwrap();
        let mut temp_shards_writer = self.temp_shards.write().unwrap();

        for from_shard in temp_shards_writer.iter() {
            self.reconcile(from_shard, &mut parent_writer);
        }

        temp_shards_writer.clear()
    }

    pub fn reconcile_specific(&self, shard_position: Option<usize>) {
        let pos = {
            let reader = self.temp_shards.read().unwrap();
            let index = shard_position.or_else(|| reader.len().checked_sub(1));

            if let Some(index) = index {
                if let Some(shard) = reader.get(index) {
                    let mut parent_shard = self.parent_shard.write().unwrap();
                    self.reconcile(shard, &mut parent_shard);
                    index
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        self.temp_shards.write().unwrap().remove(pos);
    }
}

#[cfg(test)]
mod test {
    use crate::shard::map_shard::MapShard;
    use crate::shard::shards::data_shard::config::{DataShardConfig, TempDataShardConfig};
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::temp_map_shard::TempMapShard;
    use crate::shard::Shard;
    use crate::temp_offset_types::TempOffsetTypes;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    pub async fn test_temp_shard() {
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from("./test_cases/data"));

        if !data_path.exists() {
            std::fs::create_dir(data_path.clone().clone()).unwrap();
        }

        let ctx = MapShard::<DataShard, DataShardConfig>::new(
            data_path.clone(),
            "localdata_",
            DataShardConfig { max_offsets: None },
        );

        let parent_shard = Arc::new(RwLock::new(ctx));

        let shard = TempMapShard::<DataShard, DataShardConfig, TempDataShardConfig>::new(
            data_path.clone(),
            "tempdata_",
            parent_shard.clone(),
            TempDataShardConfig {
                max_offsets: TempOffsetTypes::Custom(Some(2)),
            },
        );

        shard.insert_row("0:Hello world".as_bytes().to_vec());

        let curr_shard_id = {
            let reader = shard.temp_shards.read().unwrap();
            assert_eq!(reader.len(), 1);
            reader.first().unwrap().get_id().clone()
        };
        // It has still not be reconciled, therefore parent doesn't contain items
        let parent_items_len = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .header
            .read()
            .unwrap()
            .get_last_offset_index();
        assert_eq!(parent_items_len, -1);

        let does_shard_still_exist = shard
            .temp_shards
            .read()
            .unwrap()
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(does_shard_still_exist);

        shard.insert_row("1:Hello Cats".as_bytes().to_vec());
        // Should reconcile automatically because the tempshard only supports 2 items per shard.
        shard.insert_row("2:Hello Dogs".as_bytes().to_vec());

        // If it reconciled, it doesn't exist anymore.
        let does_shard_still_exist = shard
            .temp_shards
            .read()
            .unwrap()
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(!does_shard_still_exist);

        // There should still be a shard available which should contain "2:Hello Dogs". This one hasn't been reconciled yet.
        assert_eq!(shard.temp_shards.read().unwrap().len(), 1);
        let item = shard
            .temp_shards
            .read()
            .unwrap()
            .first()
            .unwrap()
            .read_item_from_index(0)
            .unwrap();
        assert_eq!("2:Hello Dogs".as_bytes().to_vec(), item);

        // Now that's reconciled. Parent should have the two records inserted.
        let parent_items_len = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .header
            .read()
            .unwrap()
            .get_last_offset_index();
        assert_eq!(parent_items_len, 1);

        let parent_item_1 = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .read_item_from_index(0)
            .unwrap();
        let parent_item_2 = parent_shard
            .read()
            .unwrap()
            .current_master_shard
            .read_item_from_index(1)
            .unwrap();
        assert_eq!("0:Hello world".as_bytes().to_vec(), parent_item_1);
        assert_eq!("1:Hello Cats".as_bytes().to_vec(), parent_item_2);

        std::fs::remove_dir_all(data_path).unwrap()
    }
}
