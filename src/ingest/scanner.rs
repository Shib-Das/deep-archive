use walkdir::{WalkDir, DirEntry};
use std::path::{Path, PathBuf};
use crossbeam::channel::Sender;
use anyhow::Result;

pub fn scan_directory(root: &Path, tx: Sender<PathBuf>) -> Result<()> {
    let walker = WalkDir::new(root).into_iter();

    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry?;
        if entry.file_type().is_file() {
            // We just send the path. The receiver handles the rest.
            // Using unwrap/expect here might panic if channel is closed,
            // but in this pipeline, if the receiver dies, we probably want to stop anyway.
            // Ideally we handle the error gracefully.
            if let Err(_) = tx.send(entry.path().to_path_buf()) {
                break;
            }
        }
    }
    Ok(())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with('.'))
         .unwrap_or(false)
}
