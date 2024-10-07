mod file_descriptor;

use crate::fdm::file_descriptor::FileDescriptor;
use lru::LruCache;
use parking_lot::RwLock;
use std::cell::OnceCell;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub struct FileDescriptorManager {
    cache: RwLock<LruCache<PathBuf, Arc<FileDescriptor>>>,
    max_size: usize,
}

static FDM: OnceLock<Arc<FileDescriptorManager>> = OnceLock::new();

pub fn get_fdm() -> Arc<FileDescriptorManager> {
    FDM.get().unwrap().clone()
}

impl FileDescriptorManager {
    pub fn init(max_size: usize) {
        FDM.get_or_init(|| Arc::new(FileDescriptorManager::new(max_size)));
    }

    fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(max_size).unwrap())),
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
        return Some(descriptor);
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
}
