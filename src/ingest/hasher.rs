use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;
use sha2::{Sha256, Digest};
use memmap2::MmapOptions;
use anyhow::{Result, Context};

const MMAP_THRESHOLD: u64 = 500 * 1024 * 1024; // 500 MB

pub fn calculate_hash(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let metadata = file.metadata()?;
    let len = metadata.len();

    let mut hasher = Sha256::new();

    if len > MMAP_THRESHOLD {
        // Use memory mapping for large files
        // unsafe is required for mmap, we trust the file system not to truncate the file under our feet unexpectedly
        // preventing the process from crashing (SIGBUS) is hard in Rust without signal handling,
        // but for this task we assume standard behavior.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        hasher.update(&mmap);
    } else {
        // Standard reading for smaller files
        let mut reader = BufReader::new(file);
        let mut buffer = [0; 8192];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}
