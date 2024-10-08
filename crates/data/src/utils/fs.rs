use std::io;
use std::path::{Path, PathBuf};

pub fn list_files_with_prefix<P: AsRef<Path> + Clone>(
    directory: P,
    prefix: &str,
) -> io::Result<Vec<PathBuf>> {
    Ok(std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            if file_name.starts_with(prefix) {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>())
}
