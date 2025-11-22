use std::fs::File;
use std::io::{Write, BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use anyhow::{Result, Context, anyhow};
use tracing::info;

pub struct ModelPaths {
    pub nsfw: PathBuf,
    pub tagger: PathBuf,
}

/// Main entry point to get model paths.
/// Checks .env first, then searches filesystem if not found.
pub fn get_model_paths() -> Result<ModelPaths> {
    let env_path = Path::new(".env");

    // 1. Try to load from existing .env
    if env_path.exists() {
        if let Ok(paths) = load_from_env(env_path) {
            info!("Loaded model paths from .env");
            return Ok(paths);
        }
    }

    // 2. If not found (or invalid), search the filesystem
    info!("Models not found in .env or .env missing. Searching filesystem...");
    let nsfw = find_file("nsfw.onnx", 5)?;
    let tagger = find_file("tagger.onnx", 5)?;

    info!("Found NSFW model: {:?}", nsfw);
    info!("Found Tagger model: {:?}", tagger);

    // 3. Save to .env for next time
    save_to_env(env_path, &nsfw, &tagger)?;
    info!("Saved paths to .env");

    Ok(ModelPaths { nsfw, tagger })
}

fn find_file(filename: &str, max_depth: usize) -> Result<PathBuf> {
    // Search current directory and parents up to a limit,
    // but also recurse down into subdirectories (like 'models', 'downloads')

    // Start from current dir
    let root = std::env::current_dir()?;

    // We will search inside the current directory recursively
    let search_result = WalkDir::new(&root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name() == filename);

    if let Some(entry) = search_result {
        return Ok(entry.path().to_path_buf());
    }

    // If not found, try checking the parent directory (useful if running from a subdir)
    if let Some(parent) = root.parent() {
         let parent_result = WalkDir::new(parent)
            .max_depth(max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name() == filename);

         if let Some(entry) = parent_result {
            return Ok(entry.path().to_path_buf());
        }
    }

    Err(anyhow!("Could not find file '{}' in nearby directories.", filename))
}

fn load_from_env(path: &Path) -> Result<ModelPaths> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut nsfw = None;
    let mut tagger = None;

    for line in reader.lines() {
        let line = line?;
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "NSFW_MODEL_PATH" => nsfw = Some(PathBuf::from(value.trim())),
                "TAGGER_MODEL_PATH" => tagger = Some(PathBuf::from(value.trim())),
                _ => {}
            }
        }
    }

    if let (Some(nsfw), Some(tagger)) = (nsfw, tagger) {
        Ok(ModelPaths { nsfw, tagger })
    } else {
        Err(anyhow!("Incomplete .env file"))
    }
}

fn save_to_env(path: &Path, nsfw: &Path, tagger: &Path) -> Result<()> {
    let mut file = File::create(path).context("Failed to create .env file")?;
    writeln!(file, "NSFW_MODEL_PATH={}", nsfw.display())?;
    writeln!(file, "TAGGER_MODEL_PATH={}", tagger.display())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load_env() -> Result<()> {
        let path = PathBuf::from("test_env_file");
        let nsfw_path = PathBuf::from("/tmp/nsfw.onnx");
        let tagger_path = PathBuf::from("/tmp/tagger.onnx");

        // Test Save
        save_to_env(&path, &nsfw_path, &tagger_path)?;

        // Verify file content
        let content = fs::read_to_string(&path)?;
        assert!(content.contains("NSFW_MODEL_PATH=/tmp/nsfw.onnx"));
        assert!(content.contains("TAGGER_MODEL_PATH=/tmp/tagger.onnx"));

        // Test Load
        let loaded = load_from_env(&path)?;
        assert_eq!(loaded.nsfw, nsfw_path);
        assert_eq!(loaded.tagger, tagger_path);

        // Cleanup
        fs::remove_file(path)?;

        Ok(())
    }
}
