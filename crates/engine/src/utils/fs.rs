use walkdir::DirEntry;

pub fn is_js_or_ts(entry: &DirEntry) -> bool {
    entry
        .path()
        .extension()
        .map_or(false, |ext| ext == "js" || ext == "ts")
}
