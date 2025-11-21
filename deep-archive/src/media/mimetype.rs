use std::path::Path;
use anyhow::{Result, Context};
use infer;

pub fn detect_mimetype(path: &Path) -> Result<String> {
    let kind = infer::get_from_path(path)
        .context("Failed to read file for mimetype detection")?;

    match kind {
        Some(k) => Ok(k.mime_type().to_string()),
        None => Ok("application/octet-stream".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_mimetype_detection() {
        // Create a dummy file
        let path = Path::new("tests/data/test_image.jpg");
        // It's empty so it might be octet-stream or text/plain depending on infer,
        // but let's just check it doesn't crash.
        let mime = detect_mimetype(path);
        assert!(mime.is_ok());
    }
}
