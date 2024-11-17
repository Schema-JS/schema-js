mod error;
mod iterator;
mod operations;

use crate::commit_log::error::CommitLogError;
use crate::commit_log::iterator::CommitLogIterator;
use crate::commit_log::operations::CommitLogEntry;
use crate::cursor::Cursor;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CommitLog {
    mmap: MmapMut,
    size: usize,
    write_offset: AtomicUsize,
}

impl CommitLog {
    pub fn new<P: AsRef<Path> + Clone>(log_file_path: P, size: usize) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(log_file_path)
            .unwrap();

        file.set_len(size as u64).unwrap();

        // Memory-map the file
        let mmap = unsafe { MmapMut::map_mut(&file).unwrap() };

        Self {
            mmap,
            write_offset: AtomicUsize::new(0),
            size,
        }
    }

    fn reserve_space(&self, size: usize) -> Option<usize> {
        // Atomically reserve space
        let offset = self.write_offset.fetch_add(size, Ordering::SeqCst);

        if offset + size > self.size {
            None
        } else {
            Some(offset)
        }
    }

    fn write(&self, data: CommitLogEntry) -> Result<(), CommitLogError> {
        let data_bytes = data.to_vec();
        let len = data_bytes.len();

        let offset = self.reserve_space(len).ok_or(CommitLogError::OutOfMemory)?;

        unsafe {
            // Access the mmap memory as a raw pointer
            let mmap_ptr = self.mmap.as_ptr() as *mut u8;

            // Write data into the reserved region using raw pointer arithmetic
            std::ptr::copy_nonoverlapping(data_bytes.as_ptr(), mmap_ptr.add(offset), len);
        }

        Ok(())
    }

    pub fn get_cursor(&self) -> Cursor {
        Cursor::mmap_mut(&self.mmap)
    }

    fn flush(&self) -> Result<(), std::io::Error> {
        self.mmap.flush()
    }
}

#[cfg(test)]
mod test {
    use crate::commit_log::iterator::CommitLogIterator;
    use crate::commit_log::operations::CommitLogEntry;
    use crate::commit_log::CommitLog;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::thread;

    #[tokio::test]
    pub async fn test_concurrency_commit_log() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/commit-log.bin".to_string());

        let log = CommitLog::new(fake_partial_folder_path.clone(), 1024 * 1024);
        let log = Arc::new(log);
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let log = log.clone();
                thread::spawn(move || {
                    let entry = format!("{}", i);
                    let data = entry.as_bytes();
                    log.write(CommitLogEntry::raw(data)).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        log.flush().unwrap();
        println!("All threads have finished writing.");
        let log = CommitLog::new(fake_partial_folder_path.clone(), 1024 * 1024);
        let mut cursor = log.get_cursor();
        let iter = CommitLogIterator::new(&mut cursor);
        let mut items: Vec<String> = iter
            .map(|e| {
                String::from_utf8(e.as_valid().unwrap().data.as_raw().unwrap().to_owned()).unwrap()
            })
            .collect();
        items.sort();
        assert_eq!(items.len(), 100);
        assert_eq!(items[0], "0");
        assert_eq!(items[99], "99");

        std::fs::remove_file(fake_partial_folder_path).unwrap()
    }
}
