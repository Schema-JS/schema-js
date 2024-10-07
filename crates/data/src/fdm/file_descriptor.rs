use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::Arc;

pub struct FileDescriptor {
    pub file: Arc<RwLock<File>>,
}

impl FileDescriptor {
    pub fn new_from_path<P: AsRef<Path> + Clone>(path: P) -> std::io::Result<Self> {
        let load_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path.clone())?;

        Ok(Self {
            file: Arc::new(RwLock::new(load_file)),
        })
    }
}
