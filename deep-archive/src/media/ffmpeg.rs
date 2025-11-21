use std::process::{Command, Stdio};
use std::io::Read;
use std::path::Path;
use anyhow::{Result, Context, anyhow};

pub fn extract_frames(input_path: &Path) -> Result<Vec<u8>> {
    // Arguments: -i input_file -vf fps=1/5,scale=224:224 -f rawvideo -pix_fmt rgb24 -
    // Note: fps=1/5 means 1 frame every 5 seconds.
    // scale=224:224 is for the AI model.
    // rgb24 matches the expected input for many image models (though we might need to verify HWC vs CHW).
    // The previous ML code expects CHW for normalization, but we read packed RGB here.
    // The caller or the ML pipeline logic will handle the conversion.

    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-vf")
        .arg("fps=1/5,scale=224:224")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("rgb24")
        .arg("-")
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Suppress stderr unless debugging
        .spawn()
        .context("Failed to spawn ffmpeg command")?;

    let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
    let mut buffer = Vec::new();
    stdout.read_to_end(&mut buffer).context("Failed to read ffmpeg output")?;

    let status = child.wait().context("Failed to wait on ffmpeg")?;
    if !status.success() {
        return Err(anyhow!("ffmpeg exited with non-zero status"));
    }

    Ok(buffer)
}
