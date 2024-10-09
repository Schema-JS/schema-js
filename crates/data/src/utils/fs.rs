use std::fs::File;
use std::io;
use std::io::{Seek, SeekFrom, Write};
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
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

pub fn write_at(file: &mut File, buf: &[u8], offset: u64) -> io::Result<usize> {
    #[cfg(target_family = "unix")]
    {
        file.write_at(buf, offset)
    }

    #[cfg(target_family = "windows")]
    {
        let _ = file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf).map(|e| buf.len())
    }
}
