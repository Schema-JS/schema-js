use crate::errors::ShardErrors;
use crate::fdm::FileDescriptorManager;
use crate::shard::map_shard::MapShard;
use crate::shard::{Shard, ShardConfig, TempShardConfig};
use parking_lot::RwLock;
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct DataWithIndex {
    pub data: Vec<u8>,
    pub index: u64,
}

pub struct OnReconcileCb {
    func: Option<Box<dyn Fn(Vec<DataWithIndex>) -> Result<(), ()> + Send + Sync + 'static>>,
}

impl Debug for OnReconcileCb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function pointer")
    }
}

#[derive(Debug)]
pub struct TempMapShard<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>> {
    folder: PathBuf,
    prefix: String,
    parent_shard: Arc<RwLock<MapShard<S, Opts>>>,
    pub temp_shards: Vec<S>,
    temp_opts: TempOpts,
    on_reconcile: OnReconcileCb,
    fdm: Arc<FileDescriptorManager>,
}

impl<S: Shard<Opts>, Opts: ShardConfig, TempOpts: TempShardConfig<Opts>>
    TempMapShard<S, Opts, TempOpts>
{
    pub fn new(
        folder: PathBuf,
        prefix: &str,
        parent_shard: Arc<RwLock<MapShard<S, Opts>>>,
        temp_opts: TempOpts,
        fdm: Arc<FileDescriptorManager>,
    ) -> Self {
        TempMapShard {
            parent_shard,
            folder,
            prefix: prefix.to_string(),
            temp_shards: vec![],
            temp_opts,
            on_reconcile: OnReconcileCb { func: None },
            fdm,
        }
    }

    pub fn set_on_reconcile(
        &mut self,
        data: Box<dyn Fn(Vec<DataWithIndex>) -> Result<(), ()> + Send + Sync + 'static>,
    ) {
        self.on_reconcile = OnReconcileCb { func: Some(data) };
    }

    fn create_shard(&self) -> S {
        let shard_path = self.folder.join(format!(
            "{}{}",
            self.prefix.clone(),
            Uuid::new_v4().to_string()
        ));

        S::new(
            shard_path,
            self.temp_opts.to_config(),
            None,
            self.fdm.clone(),
        )
    }

    pub fn raw_insert_rows(&mut self, data: &[&[u8]]) -> Result<u64, ShardErrors> {
        let find_usable_shard = { self.temp_shards.iter().position(|i| i.has_space()) };

        let shard_index = match find_usable_shard {
            None => {
                self.reconcile_specific(None);
                let shard = self.create_shard();
                self.temp_shards.push(shard);
                self.temp_shards.len() - 1
            }
            Some(shard) => shard,
        };

        {
            self.temp_shards
                .get(shard_index)
                .ok_or(ShardErrors::UnknownShard)?
                .insert_item(data)
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
    fn call_on_reconcile(&self, data: Vec<DataWithIndex>) -> Result<(), ()> {
        match &self.on_reconcile.func {
            None => Ok(()),
            Some(cb) => cb(data),
        }
    }

    fn reconcile(&self, from: &S, target: &mut MapShard<S, Opts>) {
        let (shard, indexes) = Self::get_reconciliation_data(from);
        let mut reconciling_items = vec![];
        // TODO: What if the row is inserted `target.insert_rows` but, the reconciling (call_on_reconcile) fails?
        for item_index in indexes {
            let binary_item = shard.read_item_from_index(item_index as usize).unwrap();
            let pos = target.insert_rows(&[&binary_item]);
            reconciling_items.push(DataWithIndex {
                data: binary_item,
                index: pos as u64,
            });
        }
        self.call_on_reconcile(reconciling_items).unwrap();
    }

    pub fn reconcile_all(&mut self) {
        let mut parent_writer = self.parent_shard.write();

        for from_shard in self.temp_shards.iter() {
            self.reconcile(from_shard, &mut parent_writer);
        }

        let paths: Vec<PathBuf> = self.temp_shards.iter().map(|i| i.get_path()).collect();
        self.fdm.remove_paths(paths);

        self.temp_shards.clear()
    }

    pub fn reconcile_specific(&mut self, shard_position: Option<usize>) {
        let pos = {
            let index = shard_position.or_else(|| self.temp_shards.len().checked_sub(1));

            if let Some(index) = index {
                if let Some(shard) = self.temp_shards.get(index) {
                    let mut parent_shard = self.parent_shard.write();
                    self.reconcile(shard, &mut parent_shard);
                    index
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        self.temp_shards.remove(pos);
    }
}

#[cfg(test)]
mod test {
    use crate::fdm::FileDescriptorManager;
    use crate::shard::map_shard::MapShard;
    use crate::shard::shards::data_shard::config::{DataShardConfig, TempDataShardConfig};
    use crate::shard::shards::data_shard::shard::DataShard;
    use crate::shard::temp_map_shard::TempMapShard;
    use crate::shard::Shard;
    use crate::temp_offset_types::TempOffsetTypes;
    use parking_lot::RwLock;
    use std::path::PathBuf;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_temp_shard() {
        let data_path = format!("./test_cases/data/{}", Uuid::new_v4().to_string());
        let data_path = std::env::current_dir()
            .unwrap()
            .join(PathBuf::from(data_path.as_str()));

        if !data_path.exists() {
            std::fs::create_dir(data_path.clone().clone()).unwrap();
        }

        let ctx = MapShard::<DataShard, DataShardConfig>::new(
            data_path.clone(),
            "localdata_",
            DataShardConfig { max_offsets: None },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        let parent_shard = Arc::new(RwLock::new(ctx));

        let mut shard = TempMapShard::<DataShard, DataShardConfig, TempDataShardConfig>::new(
            data_path.clone(),
            "tempdata_",
            parent_shard.clone(),
            TempDataShardConfig {
                max_offsets: TempOffsetTypes::Custom(Some(2)),
            },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        shard
            .raw_insert_rows(&[&"0:Hello world".as_bytes().to_vec()])
            .unwrap();

        let curr_shard_id = {
            assert_eq!(shard.temp_shards.len(), 1);
            shard.temp_shards.first().unwrap().get_id().clone()
        };
        // It has still not be reconciled, therefore parent doesn't contain items
        let parent_items_len = parent_shard
            .read()
            .current_master_shard
            .header
            .read()
            .get_last_offset_index();
        assert_eq!(parent_items_len, -1);

        let does_shard_still_exist = shard
            .temp_shards
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(does_shard_still_exist);

        shard
            .raw_insert_rows(&[&"1:Hello Cats".as_bytes().to_vec()])
            .unwrap();
        // Should reconcile automatically because the tempshard only supports 2 items per shard.
        let a = shard
            .raw_insert_rows(&[&"2:Hello Dogs".as_bytes().to_vec()])
            .unwrap();

        // If it reconciled, it doesn't exist anymore.
        let does_shard_still_exist = shard
            .temp_shards
            .iter()
            .any(|i| i.get_id() == curr_shard_id);
        assert!(!does_shard_still_exist);

        // There should still be a shard available which should contain "2:Hello Dogs". This one hasn't been reconciled yet.
        assert_eq!(shard.temp_shards.len(), 1);
        let _temp_shard = shard.temp_shards.first().unwrap();
        println!("{}", _temp_shard.get_last_index());
        let item = _temp_shard.read_item_from_index(0).unwrap();
        assert_eq!("2:Hello Dogs".as_bytes().to_vec(), item);

        // Now that's reconciled. Parent should have the two records inserted.
        let parent_items_len = parent_shard
            .read()
            .current_master_shard
            .header
            .read()
            .get_last_offset_index();
        assert_eq!(parent_items_len, 1);

        let parent_item_1 = parent_shard
            .read()
            .current_master_shard
            .read_item_from_index(0)
            .unwrap();
        let parent_item_2 = parent_shard
            .read()
            .current_master_shard
            .read_item_from_index(1)
            .unwrap();
        assert_eq!("0:Hello world".as_bytes().to_vec(), parent_item_1);
        assert_eq!("1:Hello Cats".as_bytes().to_vec(), parent_item_2);

        std::fs::remove_dir_all(data_path).unwrap()
    }
}
