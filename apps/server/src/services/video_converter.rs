use anyhow::{Context, Result};
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{error, info};

use crate::services::storage::StorageService;

/// Converts raw video bytes (mp4, webm) to an animated WebP using ffmpeg.
/// The caller is responsible for ensuring ffmpeg is installed.
pub async fn convert_video_bytes_to_webp(bytes: &[u8]) -> Result<Vec<u8>> {
    let temp_input = NamedTempFile::new().context("Failed to create temp input file")?;
    let temp_input_path = temp_input.path().to_path_buf();
    fs::write(&temp_input_path, bytes)
        .await
        .context("Failed to write temp video")?;

    convert_video_input_to_webp(temp_input_path.to_str().unwrap()).await
}

async fn convert_video_input_to_webp(input: &str) -> Result<Vec<u8>> {
    let temp_webp = tempfile::Builder::new()
        .suffix(".webp")
        .tempfile()
        .context("Failed to create temp WebP file")?;
    let temp_webp_path = temp_webp.path().to_path_buf();

    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input,
            "-vf",
            "fps=15,scale=480:-1:flags=lanczos",
            "-c:v",
            "libwebp",
            "-lossless",
            "0",
            "-quality",
            "75",
            "-loop",
            "0",
            "-preset",
            "default",
            "-an",
            temp_webp_path.to_str().unwrap(),
        ])
        .output()
        .await
        .context("Failed to execute FFmpeg — is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("FFmpeg video→WebP conversion failed: {}", stderr);
        anyhow::bail!("FFmpeg conversion failed");
    }

    let webp_data = fs::read(&temp_webp_path)
        .await
        .context("Failed to read temp WebP file")?;

    Ok(webp_data)
}

/// Downloads a video URL, converts it to animated WebP, and uploads to B2.
/// Returns the CDN URL of the stored WebP.
pub async fn convert_and_upload_video(url: &str, storage: &StorageService) -> Result<String> {
    let webp_bytes = if is_hls_url(url) {
        info!("Converting HLS video from {}", url);
        convert_video_input_to_webp(url).await?
    } else {
        info!("Downloading video from {} for WebP conversion", url);
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; memeBucketBot/1.0)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        let video_bytes = client
            .get(url)
            .send()
            .await
            .context("Failed to download video")?
            .bytes()
            .await
            .context("Failed to read video bytes")?;

        convert_video_bytes_to_webp(&video_bytes).await?
    };

    storage
        .upload_bytes(url, webp_bytes, "webp")
        .await
        .map_err(|e| anyhow::anyhow!("B2 upload failed: {e}"))
}

pub fn is_video_url(url: &str) -> bool {
    let base = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    base.ends_with(".mp4") || base.ends_with(".webm") || base.ends_with(".m3u8")
}

fn is_hls_url(url: &str) -> bool {
    let base = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    base.ends_with(".m3u8")
}

#[cfg(test)]
mod tests {
    use super::is_video_url;

    #[test]
    fn is_video_url_accepts_direct_files_and_hls_playlists() {
        assert!(is_video_url("https://example.com/video.mp4"));
        assert!(is_video_url("https://example.com/video.webm?token=1"));
        assert!(is_video_url("https://example.com/video.m3u8?token=1"));
    }

    #[test]
    fn is_video_url_rejects_non_video_urls() {
        assert!(!is_video_url("https://example.com/image.gif"));
    }
}
