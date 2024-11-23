use enum_as_inner::EnumAsInner;
use parking_lot::RwLock;
use schemajs_data::commit_log::collection::CommitLogCollection;
use schemajs_data::commit_log::iterator::{CommitLogIterator, CommitLogIteratorItem};
use schemajs_data::commit_log::operations::{CommitLogEntry, OperationData};
use schemajs_data::commit_log::reconciliation_file::ReconciliationFile;
use schemajs_data::shard::insert_item::InsertItem;
use schemajs_data::shard::map_shard::MapShard;
use schemajs_data::shard::shards::data_shard::config::DataShardConfig;
use schemajs_data::shard::shards::data_shard::shard::DataShard;
use schemajs_data::U64_SIZE;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use thiserror::Error;
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

#[derive(Debug, Serialize, Deserialize, Clone, EnumAsInner)]
pub enum TableCommitStatus {
    Succeeded,
    Partial(Vec<usize>),
}

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum TableCommitError {
    #[error("I/O Error during table commit")]
    IoError,
    #[error("Reconciliation error")]
    ReconcileError,
}

#[derive(Debug)]
pub struct TableCommitLogCollection {
    pub main_shard: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
    pub commit_log_collection: Arc<CommitLogCollection>,
    on_reconcile: OnReconcileCb,
    pub reconciliation_file: Arc<RwLock<ReconciliationFile>>,
    min_bytes_master_merge: usize,
}

impl TableCommitLogCollection {
    pub fn new(
        target_shard: Arc<RwLock<MapShard<DataShard, DataShardConfig>>>,
        folder: PathBuf,
        prefix: &str,
        max_logs: usize,
        log_size: usize,
        min_bytes_master_merge: usize,
    ) -> Self {
        Self {
            reconciliation_file: Arc::new(ReconciliationFile::new(folder.join("reconcile.data"))),
            main_shard: target_shard,
            commit_log_collection: Arc::new(CommitLogCollection::new(
                folder, prefix, max_logs, log_size,
            )),
            on_reconcile: OnReconcileCb { func: None },
            min_bytes_master_merge,
        }
    }

    pub fn log(&self, data: &[CommitLogEntry]) -> Result<TableCommitStatus, TableCommitError> {
        let mut used_commit_logs = vec![];
        let mut uncommitted_items = vec![];

        for (index, item) in data.iter().enumerate() {
            while self
                .commit_log_collection
                .waiting_for_reconciliation
                .load(Ordering::Acquire)
            {
                // Optionally, add a short sleep to avoid high CPU usage
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let commit_log_i = self.commit_log_collection.write(item); // Todo: what if error?

            match commit_log_i {
                Ok(commit_log_i) => used_commit_logs.push(commit_log_i),
                Err(_) => uncommitted_items.push(index),
            };
        }

        for i in used_commit_logs {
            let e = self.commit_log_collection.logs.read()[i].flush();
            if e.is_err() {
                return Err(TableCommitError::IoError);
            }
            // TODO: what if panics?
        }

        if uncommitted_items.is_empty() {
            Ok(TableCommitStatus::Succeeded)
        } else {
            Ok(TableCommitStatus::Partial(uncommitted_items))
        }
    }

    pub fn set_on_reconcile(
        &mut self,
        data: Box<dyn Fn(Vec<DataWithIndex>) -> Result<(), ()> + Send + Sync + 'static>,
    ) {
        self.on_reconcile = OnReconcileCb { func: Some(data) };
    }

    pub fn insert(&self, data: &[&[u8]]) -> Result<TableCommitStatus, TableCommitError> {
        let entries: Vec<CommitLogEntry> = data
            .iter()
            .map(|&data| CommitLogEntry::insert(data))
            .collect();
        self.log(&entries)
    }

    pub fn delete(&self, data: &[u64]) -> Result<TableCommitStatus, TableCommitError> {
        let entries: Vec<CommitLogEntry> = data
            .iter()
            .map(|&global| CommitLogEntry::delete(global))
            .collect();
        self.log(&entries)
    }

    pub fn reconcile(&self) -> Result<(), TableCommitError> {
        let mut reconcile_file = self.reconciliation_file.write();
        let mut main_shard = self.main_shard.write();

        // If finished is marked as false
        // When calling `reconcile`
        // It's mostly because something went wrong, the server went down, some data wasn't flushed, etc.
        // Finished should always be `true` at the time of calling this method.
        let needs_recovery = !reconcile_file.finished && reconcile_file.used;

        let (last_entry_indx, last_log_indx) = if needs_recovery {
            (
                reconcile_file.last_reconciled_entry as i64,
                reconcile_file.last_commit_log_index,
            )
        } else {
            println!("Starting reconciliation {}", main_shard.get_last_index());
            let _ = reconcile_file
                .start_reconciliation(main_shard.get_last_index())
                .unwrap();

            (-1, 0)
        };

        let logs = self.commit_log_collection.logs.read();

        let mut insert_queue = vec![];
        let mut delete_queue = vec![];
        let mut update_queue = vec![];

        let mut bytes_in_queue = 0;

        for (index, log) in logs.iter().enumerate() {
            if index < last_log_indx {
                continue;
            }

            reconcile_file.set_last_commit_log_index(index as u64);
            let _ = reconcile_file
                .flush()
                .map_err(|_| TableCommitError::ReconcileError)?;

            let mut commit_log_cursor = log.get_cursor();
            println!("Commit log cursor len {}", commit_log_cursor.len);
            for (log_index, log) in CommitLogIterator::new(&mut commit_log_cursor).enumerate() {
                println!("Commit log entry #{}.{} : {:?}", index, log_index, log);
                if last_entry_indx >= 0 && log_index <= last_entry_indx as usize {
                    continue;
                }

                match log {
                    CommitLogIteratorItem::Valid(entry) => match entry.data {
                        OperationData::Insert { data } => {
                            bytes_in_queue += data.len();
                            insert_queue.push((data, entry.id));
                        }
                        OperationData::Delete { global_indx } => {
                            delete_queue.push(global_indx);
                            bytes_in_queue += U64_SIZE;
                        }
                        OperationData::Update {
                            new_data,
                            global_indx,
                        } => {
                            bytes_in_queue += new_data.len() + U64_SIZE;
                            update_queue.push((new_data, global_indx));
                        }
                        OperationData::Raw { .. } => {}
                    },
                    CommitLogIteratorItem::Broken => continue,
                }

                if bytes_in_queue >= self.min_bytes_master_merge {
                    Self::merge_changes(
                        &mut main_shard,
                        &mut insert_queue,
                        &mut delete_queue,
                        &mut update_queue,
                        &mut reconcile_file,
                        log_index as u64,
                        needs_recovery,
                    )?;
                    bytes_in_queue = 0;
                }
            }
        }

        if bytes_in_queue > 0 {
            Self::merge_changes(
                &mut main_shard,
                &mut insert_queue,
                &mut delete_queue,
                &mut update_queue,
                &mut reconcile_file,
                0,
                needs_recovery,
            )?;
        }

        reconcile_file
            .stop_reconciliation(main_shard.get_last_index())
            .unwrap();

        Ok(())
    }

    fn merge_changes(
        main_shard: &mut MapShard<DataShard, DataShardConfig>,
        insert_queue: &mut Vec<(Vec<u8>, Uuid)>,
        delete_queue: &mut Vec<u64>,
        update_queue: &mut Vec<(Vec<u8>, u64)>,
        reconciliation_file: &mut ReconciliationFile,
        log_index: u64,
        is_recovering: bool,
    ) -> Result<(), TableCommitError> {
        // Inserts
        {
            let mut inserts: Vec<InsertItem> = insert_queue
                .iter()
                .map(|(v, i)| InsertItem::new(v.as_slice(), i.clone()))
                .collect();

            if is_recovering {
                let last_global_index = reconciliation_file.last_global_index;

                // Fetch known IDs in the range from the main shard
                let known_latest_ids = main_shard
                    .get_shard_item_ids_range(last_global_index..last_global_index + inserts.len());

                // Filter out inserts that are already in the shard
                inserts.retain(|e| !known_latest_ids.contains(&e.uuid));
            }

            if !inserts.is_empty() {
                // Insert remaining rows into the shard
                main_shard.insert_rows(&inserts);
            }

            // Clear the insert queue after processing
            insert_queue.clear();
        }

        // Deletes
        {
            let deletes: Vec<u64> = delete_queue.iter().map(|e| *e).collect();

            if !deletes.is_empty() {
                main_shard.delete_items(&deletes);
            }

            delete_queue.clear();
        }

        // Updates
        {
            let updates: Vec<(u64, &[u8])> =
                update_queue.iter().map(|e| (e.1, e.0.as_slice())).collect();

            if !updates.is_empty() {
                main_shard.update_items(updates);
            }

            update_queue.clear();
        }

        reconciliation_file.set_last_reconciled_entry(log_index);

        // unwrap is NoOp bc flush is false
        println!("Last global index {}", main_shard.get_last_index());
        reconciliation_file
            .set_last_global_index(main_shard.get_last_index(), false)
            .unwrap();

        let _ = reconciliation_file
            .flush()
            .map_err(|_| TableCommitError::ReconcileError)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::managers::single::table_commit_log_collection::TableCommitLogCollection;
    use parking_lot::RwLock;
    use schemajs_data::commit_log::operations::CommitLogEntry;
    use schemajs_data::fdm::FileDescriptorManager;
    use schemajs_data::shard::map_shard::MapShard;
    use schemajs_data::shard::shards::data_shard::config::DataShardConfig;
    use schemajs_data::shard::shards::data_shard::shard::DataShard;
    use std::path::PathBuf;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_folder() -> PathBuf {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join(format!("./test_cases/data/{}", Uuid::new_v4().to_string()));
        std::fs::create_dir(&fake_partial_folder_path).unwrap();

        fake_partial_folder_path
    }

    fn create_map_shard(folder_path: &PathBuf) -> MapShard<DataShard, DataShardConfig> {
        let mut context = MapShard::<DataShard, DataShardConfig>::new(
            folder_path.clone(),
            "data_",
            DataShardConfig {
                max_offsets: Some(35),
            },
            Arc::new(FileDescriptorManager::new(2500)),
        );

        context
    }

    #[test]
    pub fn test_commit_log_collection() {
        let data_folder = create_folder();
        let create_shard = create_map_shard(&data_folder);
        let collection = TableCommitLogCollection::new(
            Arc::new(RwLock::new(create_shard)),
            data_folder.clone(),
            "log",
            2,
            1015, // 1008 is because `entry_len_without_data` is 28. assuming an item of 1byte. Only for the sake of testing
            1015,
        );

        let entry = CommitLogEntry::insert(b"1");
        let entry_vec = entry.to_vec();
        let entry_len = entry_vec.len();
        let entry_len_without_data = entry_len - 1;
        assert_eq!(entry_len_without_data, 28);

        assert_eq!(collection.commit_log_collection.logs.read().len(), 1);

        // Insert 1015 bytes ( 1015 / 29 = 35)
        let mut items = vec![];
        for item in 0..35 {
            let log = CommitLogEntry::insert(b"1"); // 1 just to represent 1 byte. Not that it really matters honestly
            items.push(log);
        }
        collection.log(&items).unwrap();
        assert_eq!(collection.commit_log_collection.logs.read().len(), 1);

        // Now it should exceed 1015 and should create a new log
        collection.log(&[CommitLogEntry::insert(b"1")]).unwrap();
        assert_eq!(collection.commit_log_collection.logs.read().len(), 2);
    }

    #[test]
    pub fn test_commit_log_collection_reconcile() {
        let data_folder = create_folder();
        let create_shard = create_map_shard(&data_folder);
        let shard = Arc::new(RwLock::new(create_shard));
        let collection = TableCommitLogCollection::new(
            shard.clone(),
            data_folder.clone(),
            "log",
            2,
            1015, // 1008 is because `entry_len_without_data` is 28. assuming an item of 1byte. Only for the sake of testing
            1015,
        );

        // Insert 1015 bytes ( 1015 / 29 = 35)
        let mut items = vec![];
        for item in 0..35 {
            let log = CommitLogEntry::insert(b"1"); // 1 just to represent 1 byte. Not that it really matters honestly
            items.push(log);
        }
        collection.log(&items).unwrap();
        collection.log(&[CommitLogEntry::insert(b"1")]).unwrap();

        {
            let reconciliation_file = collection.reconciliation_file.read();
            assert_eq!(reconciliation_file.used, false);
            assert_eq!(reconciliation_file.last_global_index, 0);
            assert_eq!(reconciliation_file.last_commit_log_index, 0);
            assert_eq!(reconciliation_file.last_reconciled_entry, 0);
        }

        collection.reconcile().unwrap();

        {
            let reconciliation_file = collection.reconciliation_file.read();
            println!("{:?}", reconciliation_file.mmap.to_vec());
            assert_eq!(reconciliation_file.used, true);
            assert_eq!(reconciliation_file.last_global_index, 35);
            assert_eq!(reconciliation_file.last_commit_log_index, 1);
            assert_eq!(reconciliation_file.last_reconciled_entry, 1);
        }
    }
}
