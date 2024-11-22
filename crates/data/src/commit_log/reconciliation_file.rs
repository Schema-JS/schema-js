use crate::cursor::Cursor;
use crate::U64_SIZE;
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug)]
pub struct ReconciliationFile {
    pub last_commit_log_index: usize,
    pub last_reconciled_entry: usize,
    pub finished: bool,
    pub mmap: MmapMut,
}

impl ReconciliationFile {
    pub fn new<P: AsRef<Path> + Clone>(log_file_path: P) -> RwLock<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(log_file_path)
            .unwrap();

        // last_commit_log_index + last_reconciled_entry + finished
        let len = U64_SIZE + U64_SIZE + 1;

        file.set_len(len as u64).unwrap();

        // Memory-map the file
        let mut mmap = unsafe { MmapMut::map_mut(&file).unwrap() };

        let mut cursor = Cursor::mmap_mut(&mmap);
        let last_commit_log_index_bytes = cursor.consume(U64_SIZE).unwrap();
        let last_reconciled_entry_bytes = cursor.consume(U64_SIZE).unwrap();
        let finished = cursor.consume(1).unwrap();

        RwLock::new(Self {
            last_commit_log_index: u64::from_le_bytes(
                last_commit_log_index_bytes.try_into().unwrap(),
            ) as usize,
            last_reconciled_entry: u64::from_le_bytes(
                last_reconciled_entry_bytes.try_into().unwrap(),
            ) as usize,
            finished: finished[0] == 1u8,
            mmap,
        })
    }

    fn reconcile(&mut self, finished: bool) -> std::io::Result<()> {
        self.finished = finished;
        self.mmap[16] = if finished { 1u8 } else { 0u8 };
        self.mmap.flush()
    }

    pub fn start_reconciliation(&mut self) -> std::io::Result<()> {
        self.reconcile(false)
    }

    pub fn stop_reconciliation(&mut self) -> std::io::Result<()> {
        self.reconcile(true)
    }

    pub fn set_last_commit_log_index(&mut self, index: u64) {
        self.last_commit_log_index = index as usize;
        self.mmap[0..U64_SIZE].copy_from_slice(&index.to_le_bytes());
    }

    pub fn set_last_reconciled_entry(&mut self, index: u64) {
        self.last_reconciled_entry = index as usize;
        self.mmap[U64_SIZE..(U64_SIZE * 2)].copy_from_slice(&index.to_le_bytes());
    }

    pub fn flush(&self) -> std::io::Result<()> {
        self.mmap.flush()
    }
}

#[cfg(test)]
mod reconciliation_file {
    use crate::commit_log::reconciliation_file::ReconciliationFile;

    #[test]
    pub fn test_file() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/reconcile.data".to_string());

        let delete_file = std::fs::remove_file(&fake_partial_folder_path);

        let reconcile_file = ReconciliationFile::new(fake_partial_folder_path);
        {
            let mut writer = reconcile_file.write();
            assert_eq!(writer.last_commit_log_index, 0);
            assert_eq!(writer.last_reconciled_entry, 0);
            writer.set_last_commit_log_index(25);
            writer.set_last_reconciled_entry(130);
            writer.flush().unwrap();
        }

        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/reconcile.data".to_string());

        let reconcile_file = ReconciliationFile::new(fake_partial_folder_path);
        let read = reconcile_file.read();
        assert_eq!(read.last_commit_log_index, 25);
        assert_eq!(read.last_reconciled_entry, 130);
    }

    #[test]
    pub fn test_reconcile_finish() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/reconcile-finish.data".to_string());

        let delete_file = std::fs::remove_file(&fake_partial_folder_path);

        let reconcile_file = ReconciliationFile::new(fake_partial_folder_path);
        {
            let mut writer = reconcile_file.write();
            assert_eq!(writer.finished, false);
            writer.start_reconciliation().unwrap();
            assert_eq!(writer.finished, false);
        }

        {
            let fake_partial_folder_path = std::env::current_dir()
                .unwrap()
                .join("./test_cases/reconcile-finish.data".to_string());

            let reconcile_file = ReconciliationFile::new(fake_partial_folder_path);
            let mut write = reconcile_file.write();
            assert_eq!(write.finished, false);
            write.stop_reconciliation().unwrap();
            assert_eq!(write.finished, true);
        }

        {
            let fake_partial_folder_path = std::env::current_dir()
                .unwrap()
                .join("./test_cases/reconcile-finish.data".to_string());

            let reconcile_file = ReconciliationFile::new(fake_partial_folder_path);
            let mut read = reconcile_file.read();
            assert_eq!(read.finished, true);
        }
    }
}
