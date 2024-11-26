use crate::commit_log::error::CommitLogError;
use crate::commit_log::operations::CommitLogEntry;
use crate::commit_log::CommitLog;
use crate::utils::fs::list_files_with_prefix;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

#[derive(Debug)]
pub struct CommitLogCollection {
    max_logs: usize,
    log_size: usize,
    pub logs: RwLock<Vec<CommitLog>>,
    folder: PathBuf,
    prefix: String,
    pub waiting_for_reconciliation: AtomicBool,
}

impl CommitLogCollection {
    pub fn new(folder: PathBuf, prefix: &str, max_logs: usize, log_size: usize) -> Self {
        let mut logs = vec![];
        let mut log_files = list_files_with_prefix(&folder, prefix).unwrap();
        // Sort the files by their file names
        log_files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));

        for log_file in log_files {
            logs.push(CommitLog::new(log_file, log_size));
        }

        if logs.is_empty() {
            let log = Self::new_log(&folder, prefix, log_size);
            logs.push(log);
        }

        let locked_count = logs.iter().filter(|e| e.is_locked()).count();
        let waiting_for_reconciliation = if locked_count >= max_logs {
            true
        } else {
            false
        };

        Self {
            max_logs,
            log_size,
            logs: RwLock::new(logs),
            folder,
            prefix: prefix.to_string(),
            waiting_for_reconciliation: AtomicBool::new(waiting_for_reconciliation),
        }
    }

    pub fn reset(&self) {
        let mut logs = self.logs.write();

        {
            // Clean paths
            let paths: Vec<&PathBuf> = logs.iter().map(|e| &e.path).collect();
            for path in paths {
                let _ = std::fs::remove_file(path);
            }
        }

        // Clear
        logs.clear();

        let initial_log = Self::new_log(&self.folder, &self.prefix, self.log_size);
        logs.push(initial_log);
    }

    fn mark_as_waiting(&self, waiting: bool) {
        self.waiting_for_reconciliation
            .store(waiting, Ordering::Release);
    }

    fn new_log(folder: &PathBuf, prefix: &str, log_size: usize) -> CommitLog {
        let log_path = folder.join(format!("{}_{}", prefix, Uuid::new_v4().to_string()));

        CommitLog::new(log_path, log_size)
    }

    fn add_log(&self) -> Result<usize, CommitLogError> {
        let mut logs = self.logs.write();

        if logs.len() >= self.max_logs {
            self.mark_as_waiting(true);
            return Err(CommitLogError::MaxLogsReached);
        }

        let new_log = Self::new_log(&self.folder, &self.prefix, self.log_size);
        logs.push(new_log);

        // Return the index of the newly added log
        Ok(logs.len() - 1)
    }

    pub fn write(&self, data: &CommitLogEntry) -> Result<usize, CommitLogError> {
        // Acquire a read lock to check existing logs
        {
            let logs = self.logs.read();

            // Attempt to write to an available log
            for (i, log) in logs.iter().enumerate() {
                if !log.is_locked() {
                    return log.write(data).map(|_| i);
                }
            }
        }

        // If no log is available, add a new one and write to it
        let new_log_index = self.add_log()?; // Add a log and get its index
        let logs = self.logs.read(); // Acquire a read lock to access the new log
        logs[new_log_index].write(data)?;

        Ok(new_log_index)
    }
}
