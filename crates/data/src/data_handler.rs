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
            let new_mmap = unsafe { Self::mmap(&writer) };
            self.mmap = new_mmap?;

            Ok(cb)
        } else {
            Err(Error::new(ErrorKind::Other, "Too many files open in FDM"))
        }
    }
}

#[cfg(test)]
mod data_handler_tests {
    use memmap2::MmapOptions;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn read_mmap(path: PathBuf) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap()
    }

    #[tokio::test]
    pub async fn test_mmap() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("./test_cases/mmap.bin".to_string());

        {
            let mut file = read_mmap(fake_partial_folder_path.clone());

            file.write_all(b"Hello World").unwrap();
            // Create a mutable memory-mapped buffer.
            let mut mmap = unsafe { MmapOptions::new().map_mut(&file).unwrap() };
            mmap[0] = b"X".get(0).unwrap().clone();

            mmap.flush().unwrap();
        }

        let file = read_mmap(fake_partial_folder_path.clone());
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file).unwrap() };
        assert_eq!(mmap.to_vec(), b"Xello World".to_vec());
        std::fs::remove_file(fake_partial_folder_path).unwrap();
    }

    #[tokio::test]
    pub async fn test_mmap_2() {
        let fake_partial_folder_path = std::env::current_dir()
            .unwrap()
            .join("test_cases/mmap2.bin");

        // Ensure the test directory exists
        std::fs::create_dir_all(fake_partial_folder_path.parent().unwrap()).unwrap();

        {
            let mut file = read_mmap(fake_partial_folder_path.clone());

            // Resize the file to ensure it is at least 11 bytes long
            file.set_len(11).unwrap();

            // Write initial content to the file
            file.write_all(b"Hello").unwrap();

            // Create a mutable memory-mapped buffer
            let mut mmap = unsafe { MmapOptions::new().map_mut(&file).unwrap() };

            // Modify content via mmap
            mmap[0] = b'X';
            mmap[5..11].copy_from_slice(b"123456");

            // Flush changes to disk
            mmap.flush().unwrap();
        }

        // Verify the file content after modifications
        let file = read_mmap(fake_partial_folder_path.clone());
        let mmap = unsafe { MmapOptions::new().map_mut(&file).unwrap() };
        assert_eq!(mmap.to_vec(), b"Xello123456".to_vec());

        // Clean up the test file
        std::fs::remove_file(fake_partial_folder_path).unwrap();
    }
}
