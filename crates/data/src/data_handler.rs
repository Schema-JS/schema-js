use crate::fdm::FileDescriptorManager;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct DataHandler {
    pub path: PathBuf,
    fdm: Arc<FileDescriptorManager>,
    mmap: Mmap,
}

impl DataHandler {
    unsafe fn new_from_path<P: AsRef<Path> + Clone>(
        path: P,
        fdm: Arc<FileDescriptorManager>,
    ) -> std::io::Result<Self> {
        if let Some(descriptor) = fdm.pop_insert(&path) {
            let file = descriptor.file.read();
            Ok(Self {
                path: path.as_ref().to_path_buf(),
                fdm,
                mmap: Self::mmap(&file)?,
            })
        } else {
            Err(Error::new(ErrorKind::Other, "Too many files open in FDM"))
        }
    }

    unsafe fn mmap(file: &File) -> std::io::Result<Mmap> {
        Mmap::map(file)
    }

    #[cfg(test)]
    pub unsafe fn access_map(&self) -> &Mmap {
        &self.mmap
    }

    pub unsafe fn new<P: AsRef<Path> + Clone>(
        path: P,
        fdm: Arc<FileDescriptorManager>,
    ) -> std::io::Result<RwLock<Self>> {
        Ok(RwLock::new(Self::new_from_path(path, fdm)?))
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_bytes(&self, from: usize, to: usize) -> Option<&[u8]> {
        self.mmap.get(from..to)
    }

    pub fn read_pointer(&self, start: u64, max_bytes: usize) -> Option<Vec<u8>> {
        self.get_bytes(start as usize, start as usize + max_bytes)
            .map(|i| i.to_vec())
    }

    pub fn operate<F, R>(&mut self, callback: F) -> std::io::Result<R>
    where
        F: FnOnce(&mut File) -> std::io::Result<R>,
    {
        let fdm = self.fdm.clone();
        if let Some(fd) = fdm.get(&self.path) {
            let mut writer = fd.file.write();
            let cb = callback(&mut writer)?;

            writer.flush()?;

            #[cfg(target_os = "linux")]
            {
                writer.sync_all()?;
            }

            let new_mmap = unsafe { Self::mmap(&writer) };
            self.mmap = new_mmap?;

            Ok(cb)
        } else {
            Err(Error::new(ErrorKind::Other, "Too many files open in FDM"))
        }
    }
}
