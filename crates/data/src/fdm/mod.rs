mod file_descriptor;

use crate::fdm::file_descriptor::FileDescriptor;
use lru::LruCache;
use parking_lot::RwLock;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct FileDescriptorManager {
    cache: Arc<RwLock<LruCache<PathBuf, Arc<FileDescriptor>>>>,
    max_size: usize,
}

impl FileDescriptorManager {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(max_size).unwrap(),
            ))),
            max_size,
        }
    }

    pub fn get(&self, path: &PathBuf) -> Option<Arc<FileDescriptor>> {
        let reader = self.cache.read();
        let file_descriptor = reader.peek(path);
        if let Some(descriptor) = file_descriptor {
            return Some(descriptor.clone());
        }

        None
    }

    // Insert a new file descriptor, using try_write to avoid blocking
    fn insert<P: AsRef<Path> + Clone>(
        &self,
        path: P,
        cache: &mut LruCache<PathBuf, Arc<FileDescriptor>>,
    ) -> Option<Arc<FileDescriptor>> {
        let path_buf = path.as_ref().to_path_buf();
        let descriptor = Arc::new(FileDescriptor::new_from_path(path).ok()?);
        cache.push(path_buf, descriptor.clone());
        Some(descriptor)
    }

    // Pop a file descriptor if it's available (i.e., not busy)
    pub fn pop_if_available(&self, cache: &mut LruCache<PathBuf, Arc<FileDescriptor>>) -> bool {
        let mut candidate = None;
        {
            for (id, descriptor) in cache.iter() {
                if !descriptor.file.is_locked_exclusive() {
                    candidate = Some(id.clone());
                    break;
                }
            }
        }

        // If we found a non-busy descriptor, remove it from the cache
        if let Some(key_to_pop) = candidate {
            cache.pop(&key_to_pop); // Actually remove the descriptor from the cache
            return true;
        }

        false
    }

    pub fn pop_insert<P: AsRef<Path> + Clone>(&self, path: P) -> Option<Arc<FileDescriptor>> {
        let mut cache = self.cache.write();
        if cache.len() < self.max_size {
            self.insert(path, &mut cache)
        } else {
            let succeeded = self.pop_if_available(&mut cache);
            if succeeded {
                self.insert(path, &mut cache)
            } else {
                None
            }
        }
    }

    pub fn remove_paths(&self, paths: Vec<PathBuf>) {
        let fdm = self.cache.clone();
        if !paths.is_empty() && self.max_size >= { fdm.read().len() } {
            tokio::spawn(async move {
                let mut writer = fdm.write();
                for path in paths.iter() {
                    writer.pop(path);
                }
            });
        }
    }
}

#[cfg(test)]
mod fdm_tests {
    use crate::fdm::FileDescriptorManager;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fdm() {
        let fdm = Arc::new(FileDescriptorManager::new(3));
        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.into_path();

        let fs_1 = temp_dir_path.join("1.data");
        let fs_2 = temp_dir_path.join("2.data");
        let fs_3 = temp_dir_path.join("3.data");
        let fs_4 = temp_dir_path.join("4.data");

        assert!(fdm.pop_insert(&fs_1).is_some());
        assert!(fdm.pop_insert(&fs_2).is_some());
        assert!(fdm.pop_insert(&fs_3).is_some());
        assert_eq!(fdm.cache.read().len(), 3);

        // Use fs_2
        let fdm_2 = fdm.clone();
        let get_val = fdm_2.get(&fs_2.clone()).unwrap();
        let handle = std::thread::spawn(move || {
            let _file = get_val.file.write();
            std::thread::sleep(Duration::from_secs(5));
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        let get_fdm_2 = fdm.get(&fs_2);
        assert!(get_fdm_2.unwrap().file.is_locked_exclusive());

        assert!(fdm.pop_insert(&fs_4).is_some());
        let mut bools = [fdm.get(&fs_1).is_some(), fdm.get(&fs_3).is_some()];
        bools.sort_by(|a, b| b.cmp(a));
        assert_eq!(bools[0], true);
        assert_eq!(bools[1], false);
        assert!(fdm.get(&fs_4).is_some());
        tokio::time::sleep(Duration::from_secs(6)).await;
        let get_fdm_2 = fdm.get(&fs_2);
        assert!(get_fdm_2.is_some());
        assert!(!get_fdm_2.unwrap().file.is_locked_exclusive());
    }
}
