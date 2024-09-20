use memmap2::Mmap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug)]
pub struct DataHandler {
    pub path: PathBuf,
    mmap: Mmap,
    file: File,
}

impl DataHandler {
    unsafe fn new_from_path<P: AsRef<Path> + Clone>(path: P) -> std::io::Result<Self> {
        println!("{}", path.as_ref().to_str().unwrap().to_string());
        let load_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path.clone())
            .expect("Failed to create shard file");

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            mmap: Mmap::map(&load_file)?,
            file: load_file,
        })
    }

    #[cfg(test)]
    pub unsafe fn access_map(&self) -> &Mmap {
        &self.mmap
    }

    #[cfg(test)]
    pub unsafe fn access_file(&self) -> &File {
        &self.file
    }

    unsafe fn new_from_file(path: PathBuf, file: File) -> std::io::Result<Self> {
        Ok(Self {
            path,
            mmap: Mmap::map(&file)?,
            file,
        })
    }

    pub unsafe fn new<P: AsRef<Path> + Clone>(path: P) -> std::io::Result<RwLock<Self>> {
        Ok(RwLock::new(Self::new_from_path(path)?))
    }

    pub unsafe fn new_with_file(path: PathBuf, file: File) -> std::io::Result<RwLock<Self>> {
        Ok(RwLock::new(Self::new_from_file(path, file)?))
    }

    pub fn metadata(&self) -> std::io::Result<Metadata> {
        self.file.metadata()
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
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
        let cb = callback(&mut self.file)?;

        self.file.flush()?;

        let new_mmap = unsafe { Mmap::map(&self.file) };
        self.mmap = new_mmap?;

        Ok(cb)
    }
}
