use anyhow::{Context, Result};
use reqwest::multipart;
use serde::Deserialize;
use std::process::Command;
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{error, info};

#[derive(Deserialize)]
struct ImgBBUploadResponse {
    data: Option<ImgBBData>,
    success: bool,
}

#[derive(Deserialize)]
struct ImgBBData {
    url: String,
}

pub async fn convert_and_upload_mp4(url: &str, imgbb_api_key: &str) -> Result<String> {
    info!("Downloading MP4 from {} for conversion", url);
    let mp4_data = reqwest::get(url)
        .await
        .context("Failed to download MP4")?
        .bytes()
        .await
        .context("Failed to read MP4 bytes")?;

    let temp_mp4 = NamedTempFile::new().context("Failed to create temp MP4 file")?;
    let temp_mp4_path = temp_mp4.path().to_path_buf();
    fs::write(&temp_mp4_path, &mp4_data)
        .await
        .context("Failed to write temp MP4")?;

    let temp_gif = NamedTempFile::new().context("Failed to create temp GIF file")?;
    let temp_gif_path = temp_gif.path().to_path_buf();

    info!("Running FFmpeg to convert MP4 to GIF");
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            temp_mp4_path.to_str().unwrap(),
            "-vf",
            "fps=15,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            "-loop",
            "0",
            temp_gif_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to execute FFmpeg. Is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("FFmpeg conversion failed: {}", stderr);
        anyhow::bail!("FFmpeg conversion failed");
    }

    info!("Uploading converted GIF to ImgBB");
    let gif_data = fs::read(&temp_gif_path)
        .await
        .context("Failed to read temp GIF")?;

    let client = reqwest::Client::new();
    let form = multipart::Form::new()
        .text("key", imgbb_api_key.to_string())
        .part(
            "image",
            multipart::Part::bytes(gif_data.to_vec())
                .file_name("converted.gif")
                .mime_str("image/gif")?,
        );

    let res = client
        .post("https://api.imgbb.com/1/upload")
        .multipart(form)
        .send()
        .await
        .context("Failed to send request to ImgBB")?;

    let res_text = res.text().await?;
    let imgbb_res: ImgBBUploadResponse = serde_json::from_str(&res_text)
        .context(format!("Failed to parse ImgBB response: {}", res_text))?;

    if !imgbb_res.success || imgbb_res.data.is_none() {
        anyhow::bail!("ImgBB upload failed: {}", res_text);
    }

    Ok(imgbb_res.data.unwrap().url)
}
