pub mod collection;
pub mod error;
pub mod iterator;
pub mod operations;

pub mod reconciliation_file;

use crate::commit_log::error::CommitLogError;
use crate::commit_log::operations::CommitLogEntry;
use crate::cursor::Cursor;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

#[derive(Debug)]
pub struct CommitLog {
    mmap: MmapMut,
    size: usize,
    write_offset: AtomicUsize,
    initialized: bool,
    locked: AtomicBool,
    busy: AtomicUsize, // Tracks the number of active writes
}

pub const COMMIT_LOG_METADATA_SIZE: usize = 2;

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
        let mut mmap = unsafe { MmapMut::map_mut(&file).unwrap() };

        // Read the first two bytes
        let initialized = mmap[0] == 1u8;
        let locked = mmap[1] == 1u8;

        if !initialized {
            // Initialize the file
            mmap[0] = 1u8; // Mark as initialized
            mmap[1] = 0u8; // Mark as unlocked
            mmap.flush()
                .expect("Failed to flush mmap during initialization");
        }

        Self {
            mmap,
            write_offset: AtomicUsize::new(COMMIT_LOG_METADATA_SIZE), // It starts from 2 because [initialized, locked]
            size,
            initialized,
            locked: AtomicBool::from(locked),
            busy: AtomicUsize::new(0),
        }
    }

    fn reserve_space(&self, size: usize) -> Option<usize> {
        // Atomically reserve space
        let offset = self.write_offset.fetch_add(size, Ordering::SeqCst);

        if offset + size > self.size {
            self.set_locked(true);
            None
        } else {
            Some(offset)
        }
    }

    pub fn is_available(&self) -> bool {
        if self.locked.load(Ordering::Acquire) {
            // If locked is true, the log is permanently unavailable
            return false;
        }

        // Check if there are no active writes
        self.busy.load(Ordering::Acquire) == 0
    }

    fn write(&self, data: &CommitLogEntry) -> Result<(), CommitLogError> {
        // Check if the log is locked before proceeding
        if self.is_locked() {
            return Err(CommitLogError::LogLocked);
        }

        // Indicate the log is busy by incrementing the counter
        self.busy.fetch_add(1, Ordering::SeqCst);

        let data_bytes = data.to_vec();
        let len = data_bytes.len();

        let offset = self.reserve_space(len).ok_or_else(|| {
            self.busy.fetch_sub(1, Ordering::SeqCst); // Decrement on failure
            CommitLogError::OutOfMemory
        })?;

        if self.is_locked() {
            self.busy.fetch_sub(1, Ordering::SeqCst); // Decrement on failure
            return Err(CommitLogError::LogLocked);
        }

        unsafe {
            // Access the mmap memory as a raw pointer
            let mmap_ptr = self.mmap.as_ptr() as *mut u8;

            // Write data into the reserved region using raw pointer arithmetic
            std::ptr::copy_nonoverlapping(data_bytes.as_ptr(), mmap_ptr.add(offset), len);
        }

        Ok(())
    }

    /// Check if the log is locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Set the lock state (true for locked, false for unlocked)
    pub fn set_locked(&self, locked: bool) {
        // Update the in-memory AtomicBool
        self.locked.store(locked, Ordering::Release);

        // Update the mmap to reflect the lock state
        let lock_flag = if locked { 1u8 } else { 0u8 };

        // Unsafe write to mmap with synchronization
        unsafe {
            let lock_ptr = self.mmap.as_ptr().add(1) as *mut u8;
            std::ptr::write(lock_ptr, lock_flag);
        }

        // Flush the mmap to persist changes
        self.mmap
            .flush()
            .expect("Failed to flush mmap during lock state change");
    }

    pub fn get_cursor(&self) -> Cursor {
        Cursor::mmap_mut(&self.mmap).set_starting_pos(COMMIT_LOG_METADATA_SIZE)
    }

    pub fn flush(&self) -> Result<(), std::io::Error> {
        // Mark the log as no longer busy
        self.busy.fetch_sub(1, Ordering::SeqCst);
        self.mmap.flush()
    }
}

#[cfg(test)]
mod test {
    use crate::commit_log::error::CommitLogError;
    use crate::commit_log::iterator::CommitLogIterator;
    use crate::commit_log::operations::CommitLogEntry;
    use crate::commit_log::CommitLog;
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;

    #[tokio::test]
    pub async fn test_concurrency_commit_log() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/commit-log.bin".to_string());
        let _ = std::fs::remove_file(fake_partial_folder_path.clone());

        let log = CommitLog::new(fake_partial_folder_path.clone(), 1024 * 1024);
        let log = Arc::new(log);
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let log = log.clone();
                thread::spawn(move || {
                    let entry = format!("{}", i);
                    let data = entry.as_bytes();
                    log.write(&CommitLogEntry::raw(data)).unwrap();
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

    #[test]
    fn test_basic_locking_behavior() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/commit-log-lock.bin".to_string());
        std::fs::remove_file(fake_partial_folder_path.clone()).unwrap();

        let mut commit_log = CommitLog::new(fake_partial_folder_path, 1024);

        // Write to the log while unlocked
        let entry = CommitLogEntry::raw(&vec![1, 2, 3, 4]);
        let write = commit_log.write(&entry);
        assert!(write.is_ok());

        // Lock the log
        commit_log.set_locked(true);

        // Attempt to write while locked
        let entry = CommitLogEntry::raw(&vec![5, 6, 7, 8]);
        assert!(matches!(
            commit_log.write(&entry),
            Err(CommitLogError::LogLocked)
        ));

        // Unlock the log
        commit_log.set_locked(false);

        // Write to the log after unlocking
        let entry = CommitLogEntry::raw(&vec![9, 10, 11, 12]);
        assert!(commit_log.write(&entry).is_ok());
    }

    #[test]
    fn test_concurrent_writes_respect_lock() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/commit-log-lock-thread.bin".to_string());
        let _ = std::fs::remove_file(fake_partial_folder_path.clone());

        let commit_log = Arc::new(CommitLog::new(fake_partial_folder_path, 1024));
        let barrier = Arc::new(Barrier::new(3)); // 3 threads (main + 2 writers)

        let commit_log_clone1 = Arc::clone(&commit_log);
        let barrier_clone1 = Arc::clone(&barrier);

        let handle1 = thread::spawn(move || {
            let entry = CommitLogEntry::raw(&vec![1, 2, 3, 4]);
            barrier_clone1.wait(); // Synchronize start
            commit_log_clone1.write(&entry)
        });

        let commit_log_clone2 = Arc::clone(&commit_log);
        let barrier_clone2 = Arc::clone(&barrier);

        let handle2 = thread::spawn(move || {
            let entry = CommitLogEntry::raw(&vec![5, 6, 7, 8]);
            barrier_clone2.wait(); // Synchronize start
            commit_log_clone2.write(&entry)
        });

        // Lock the log before the threads start writing
        commit_log.set_locked(true);
        barrier.wait(); // Let threads proceed

        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        assert_eq!(result1, Err(CommitLogError::LogLocked));
        assert!(matches!(result2, Err(CommitLogError::LogLocked)));

        // Unlock the log and retry
        commit_log.set_locked(false);
        let entry = CommitLogEntry::raw(&vec![9, 10, 11, 12]);
        assert!(commit_log.write(&entry).is_ok());
    }

    #[test]
    fn test_commit_log_concurrent_write_with_space_limit() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        // Setup: Create a file path and remove any existing file
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("test_cases/commit-log-lock-thread-sized.bin");
        let _ = std::fs::remove_file(&fake_partial_folder_path);

        // Create a commit log with a small size to simulate running out of space
        let commit_log = Arc::new(CommitLog::new(fake_partial_folder_path, 28)); // Only 16 bytes available
        let barrier = Arc::new(Barrier::new(3)); // 3 threads (main + 2 writers)

        // Thread 1: Attempt to write 4 bytes
        let commit_log_clone1 = Arc::clone(&commit_log);
        let barrier_clone1 = Arc::clone(&barrier);
        let handle1 = thread::spawn(move || {
            let entry = CommitLogEntry::raw(&vec![1, 2, 3, 4]); // 4 bytes
            barrier_clone1.wait(); // Synchronize start
            commit_log_clone1.write(&entry)
        });

        // Thread 2: Attempt to write 8 bytes
        let commit_log_clone2 = Arc::clone(&commit_log);
        let barrier_clone2 = Arc::clone(&barrier);
        let handle2 = thread::spawn(move || {
            let entry = CommitLogEntry::raw(&vec![5, 6, 7, 8, 9, 10, 11, 12]); // 8 bytes
            barrier_clone2.wait(); // Synchronize start
            commit_log_clone2.write(&entry)
        });

        // Main thread waits for all threads to start
        barrier.wait();

        // Collect results from threads
        let result1 = handle1.join().expect("Thread 1 panicked");
        let result2 = handle2.join().expect("Thread 2 panicked");

        // Verify one succeeded and one failed due to space limitations
        let success_count = [&result1, &result2]
            .iter()
            .filter(|result| result.is_ok())
            .count();

        let failure_count = [&result1, &result2]
            .iter()
            .filter(|result| matches!(result, Err(CommitLogError::OutOfMemory)))
            .count();

        assert_eq!(success_count, 1, "Exactly one thread should succeed");
        assert_eq!(
            failure_count, 1,
            "Exactly one thread should fail due to out of space"
        );

        // Final assertions: Verify the log is locked if space is exhausted
        assert!(
            commit_log.is_locked(),
            "The commit log should be locked after running out of space"
        );
    }

    #[test]
    fn test_busy_increment_and_decrement() {
        // Setup: Create a file path and remove any existing file
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("test_cases/commit-log-lock-sized-busy.bin");
        let _ = std::fs::remove_file(&fake_partial_folder_path);

        let commit_log = CommitLog::new(fake_partial_folder_path, 1024);

        // Busy counter should start at 0
        assert_eq!(commit_log.busy.load(Ordering::Acquire), 0);

        // Simulate a write by incrementing the busy counter
        commit_log.busy.fetch_add(1, Ordering::SeqCst);
        assert_eq!(commit_log.busy.load(Ordering::Acquire), 1);

        // Simulate the write completing by decrementing the busy counter
        commit_log.busy.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(commit_log.busy.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_busy_with_multiple_threads() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        // Setup: Create a file path and remove any existing file
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("test_cases/commit-log-lock-sized-busy-multithread.bin");
        let _ = std::fs::remove_file(&fake_partial_folder_path);

        let commit_log = Arc::new(CommitLog::new(fake_partial_folder_path, 1024));
        let barrier = Arc::new(Barrier::new(3)); // 3 threads (main + 2 writers)

        // Thread 1: Increment busy counter
        let commit_log_clone1 = Arc::clone(&commit_log);
        let barrier_clone1 = Arc::clone(&barrier);
        let handle1 = thread::spawn(move || {
            barrier_clone1.wait(); // Synchronize start
            commit_log_clone1.busy.fetch_add(1, Ordering::SeqCst);
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(100));
            commit_log_clone1.busy.fetch_sub(1, Ordering::SeqCst);
        });

        // Thread 2: Increment busy counter
        let commit_log_clone2 = Arc::clone(&commit_log);
        let barrier_clone2 = Arc::clone(&barrier);
        let handle2 = thread::spawn(move || {
            barrier_clone2.wait(); // Synchronize start
            commit_log_clone2.busy.fetch_add(1, Ordering::SeqCst);
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(100));
            commit_log_clone2.busy.fetch_sub(1, Ordering::SeqCst);
        });

        barrier.wait(); // Allow threads to start writing

        // Both threads should increment busy, so the counter should be 2
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(commit_log.busy.load(Ordering::Acquire), 2);

        // After threads complete, the counter should return to 0
        handle1.join().unwrap();
        handle2.join().unwrap();
        assert_eq!(commit_log.busy.load(Ordering::Acquire), 0);
    }
}
