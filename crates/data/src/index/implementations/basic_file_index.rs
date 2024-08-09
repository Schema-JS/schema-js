use std::fs::File;
use std::path::PathBuf;

pub struct BasicFileIndex {
    file: File,
    index_name: String,
    folder_path: PathBuf,
    index_prefix: String,
    max_capacity: Option<u64>,
}
