use std::path::Path;
use std::process::Command;
use std::env;
use anyhow::{Result, Context, anyhow};

pub fn create_iso(source_dir: &Path, output_iso: &Path) -> Result<()> {
    // Ensure reproducible builds by setting SOURCE_DATE_EPOCH
    // We use a fixed timestamp or one provided by the user/env.
    // For this project, let's just set it to a fixed value (e.g., 0 or explicit date) if not present,
    // or just ensure it is set. The prompt implies we must set it.

    // Check if it's already set, if not, set to a deterministic value (e.g. 1704067200 for 2024-01-01)
    if env::var("SOURCE_DATE_EPOCH").is_err() {
        env::set_var("SOURCE_DATE_EPOCH", "1704067200");
    }

    // Command: xorriso -as mkisofs -o output.iso -R -J source_dir
    // -R: Rock Ridge extensions (posix perms)
    // -J: Joliet extensions (windows compatibility)
    // -V: Volume ID

    let status = Command::new("xorriso")
        .arg("-as")
        .arg("mkisofs")
        .arg("-o")
        .arg(output_iso)
        .arg("-R")
        .arg("-J")
        .arg("-V")
        .arg("DEEP_ARCHIVE")
        .arg(source_dir)
        .status()
        .context("Failed to execute xorriso command. Is it installed?")?;

    if !status.success() {
        return Err(anyhow!("xorriso exited with non-zero status"));
    }

    Ok(())
}
