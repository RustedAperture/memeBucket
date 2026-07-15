use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tokio::fs;
use tracing::{error, info};
use url::Url;

use crate::services::images::{fetch_success, read_limited_bytes};
use crate::services::storage::StorageService;

/// Per-asset cap applied when downloading HLS playlists, keys, init segments,
/// and media segments — bounds memory use while assembling a playlist locally.
const HLS_ASSET_READ_LIMIT_BYTES: usize = 20 * 1024 * 1024;
/// Master playlists can point at a variant playlist; cap the follow depth so a
/// malicious/malformed playlist chain can't recurse indefinitely.
const HLS_MAX_PLAYLIST_REDIRECTS: usize = 3;

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
            // `input` is always a path inside our own temp directory by the time
            // it reaches ffmpeg (remote HLS playlists/segments are fetched and
            // validated ourselves first) — restrict ffmpeg to the local
            // filesystem so it can never be tricked into an outbound fetch,
            // even by a URI our HLS rewriter failed to account for.
            "-protocol_whitelist",
            "file",
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
        let (_hls_dir, playlist_path) = download_hls_playlist_safely(url).await?;
        convert_video_input_to_webp(playlist_path.to_str().unwrap()).await?
    } else {
        info!("Downloading video from {} for WebP conversion", url);
        let video_bytes = fetch_asset_bytes(url).await?;
        convert_video_bytes_to_webp(&video_bytes).await?
    };

    storage
        .upload_bytes(url, webp_bytes, "webp")
        .await
        .map_err(|e| anyhow::anyhow!("B2 upload failed: {e}"))
}

/// Fetches a URL through the same SSRF-safe resolver used for image
/// submissions (DNS-pinned to a validated non-internal IP), bounding the body
/// size so a hostile server can't exhaust memory.
async fn fetch_asset_bytes(url: &str) -> Result<Vec<u8>> {
    let response = fetch_success(url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch {}: {}", url, e.user_message()))?;
    read_limited_bytes(response, HLS_ASSET_READ_LIMIT_BYTES)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", url, e.user_message()))
}

/// Safely materializes a remote HLS stream on local disk: follows at most one
/// master→variant hop, then downloads every key/init-segment/media-segment URI
/// referenced by the leaf playlist through `fetch_asset_bytes` (so each one is
/// individually SSRF-validated) and rewrites the playlist to reference the
/// local copies. ffmpeg is only ever handed this local playlist, so it never
/// makes a network request of its own — a hostile playlist cannot redirect
/// ffmpeg's segment fetches at internal hosts.
async fn download_hls_playlist_safely(start_url: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempfile::tempdir().context("Failed to create temp dir for HLS download")?;
    let mut current_url = start_url.to_string();

    for _ in 0..HLS_MAX_PLAYLIST_REDIRECTS {
        let bytes = fetch_asset_bytes(&current_url).await?;
        let text = String::from_utf8(bytes).context("HLS playlist is not valid UTF-8")?;

        match first_stream_inf_uri(&text, &current_url) {
            Some(variant_url) => current_url = variant_url,
            None => {
                let playlist_path =
                    download_and_rewrite_media_playlist(&text, &current_url, dir.path()).await?;
                return Ok((dir, playlist_path));
            }
        }
    }

    anyhow::bail!("HLS playlist nesting too deep (possible malformed or hostile playlist)")
}

/// Downloads every asset referenced by a leaf (non-master) HLS playlist and
/// rewrites the playlist to point at the local copies. Local filenames are
/// always freshly generated (`seg_0.ts`, `key_0.bin`, ...) — never derived
/// from the attacker-influenced URI — so there's no path-traversal surface.
async fn download_and_rewrite_media_playlist(
    text: &str,
    base_url: &str,
    dir: &Path,
) -> Result<PathBuf> {
    let mut segment_count = 0usize;
    let mut key_count = 0usize;
    let mut map_count = 0usize;
    let mut rewritten_lines = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim_end_matches('\r');

        if line.starts_with("#EXT-X-KEY")
            && let Some(uri) = extract_uri_attr(line)
        {
            let absolute = resolve_hls_uri(base_url, &uri)?;
            let bytes = fetch_asset_bytes(&absolute).await?;
            let filename = format!("key_{key_count}.bin");
            key_count += 1;
            fs::write(dir.join(&filename), &bytes)
                .await
                .context("Failed to write HLS key to temp dir")?;
            rewritten_lines.push(replace_uri_attr(line, &filename));
            continue;
        }

        if line.starts_with("#EXT-X-MAP")
            && let Some(uri) = extract_uri_attr(line)
        {
            let absolute = resolve_hls_uri(base_url, &uri)?;
            let bytes = fetch_asset_bytes(&absolute).await?;
            let filename = format!("map_{map_count}.mp4");
            map_count += 1;
            fs::write(dir.join(&filename), &bytes)
                .await
                .context("Failed to write HLS init segment to temp dir")?;
            rewritten_lines.push(replace_uri_attr(line, &filename));
            continue;
        }

        if line.is_empty() || line.starts_with('#') {
            rewritten_lines.push(line.to_string());
            continue;
        }

        // A plain, non-comment line in a media playlist is a segment URI.
        let absolute = resolve_hls_uri(base_url, line)?;
        let bytes = fetch_asset_bytes(&absolute).await?;
        let filename = format!("seg_{segment_count}.ts");
        segment_count += 1;
        fs::write(dir.join(&filename), &bytes)
            .await
            .context("Failed to write HLS segment to temp dir")?;
        rewritten_lines.push(filename);
    }

    if segment_count == 0 {
        anyhow::bail!("HLS playlist referenced no media segments");
    }

    let playlist_path = dir.join("playlist.m3u8");
    fs::write(&playlist_path, rewritten_lines.join("\n"))
        .await
        .context("Failed to write rewritten HLS playlist")?;

    Ok(playlist_path)
}

/// Resolves a (possibly relative) URI against the playlist's own URL and
/// rejects anything that isn't plain http/https, so a playlist can't smuggle
/// a `file://` or other scheme past our fetcher.
fn resolve_hls_uri(base_url: &str, uri: &str) -> Result<String> {
    let base = Url::parse(base_url).context("Invalid HLS playlist base URL")?;
    let resolved = base.join(uri).context("Invalid HLS URI reference")?;
    if !matches!(resolved.scheme(), "http" | "https") {
        anyhow::bail!("Unsupported HLS URI scheme: {}", resolved.scheme());
    }
    Ok(resolved.to_string())
}

/// Finds the first variant playlist URI in a master playlist
/// (`#EXT-X-STREAM-INF` followed by a URI line). Returns `None` if `text` is
/// already a leaf/media playlist.
fn first_stream_inf_uri(text: &str, base_url: &str) -> Option<String> {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));

    while let Some(line) = lines.next() {
        if !line.starts_with("#EXT-X-STREAM-INF") {
            continue;
        }
        for candidate in lines.by_ref() {
            if candidate.is_empty() || candidate.starts_with('#') {
                continue;
            }
            return resolve_hls_uri(base_url, candidate).ok();
        }
    }

    None
}

fn extract_uri_attr(line: &str) -> Option<String> {
    let needle = "URI=\"";
    let start = line.find(needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn replace_uri_attr(line: &str, new_value: &str) -> String {
    let needle = "URI=\"";
    let Some(start) = line.find(needle) else {
        return line.to_string();
    };
    let value_start = start + needle.len();
    let Some(end_rel) = line[value_start..].find('"') else {
        return line.to_string();
    };
    let end = value_start + end_rel;
    format!("{}{}{}", &line[..value_start], new_value, &line[end..])
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
    use super::*;

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

    #[test]
    fn resolve_hls_uri_joins_relative_segment_against_playlist_base() {
        let resolved = resolve_hls_uri("https://example.com/hls/playlist.m3u8", "seg0.ts").unwrap();
        assert_eq!(resolved, "https://example.com/hls/seg0.ts");
    }

    #[test]
    fn resolve_hls_uri_rejects_non_http_schemes() {
        // A hostile playlist must not be able to smuggle a file:// (or other
        // non-http) reference past the SSRF-safe fetcher.
        let result = resolve_hls_uri(
            "https://example.com/hls/playlist.m3u8",
            "file:///etc/passwd",
        );
        assert!(result.is_err());
    }

    #[test]
    fn extract_uri_attr_reads_quoted_uri_value() {
        assert_eq!(
            extract_uri_attr(r#"#EXT-X-KEY:METHOD=AES-128,URI="key.bin",IV=0x1"#),
            Some("key.bin".to_string())
        );
        assert_eq!(extract_uri_attr("#EXTINF:6.0,"), None);
    }

    #[test]
    fn replace_uri_attr_swaps_only_the_uri_value() {
        let line = r#"#EXT-X-KEY:METHOD=AES-128,URI="https://attacker.example/key",IV=0x1"#;
        assert_eq!(
            replace_uri_attr(line, "key_0.bin"),
            r#"#EXT-X-KEY:METHOD=AES-128,URI="key_0.bin",IV=0x1"#
        );
    }

    #[test]
    fn first_stream_inf_uri_finds_variant_in_master_playlist() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nvariant.m3u8\n";
        assert_eq!(
            first_stream_inf_uri(master, "https://example.com/hls/master.m3u8"),
            Some("https://example.com/hls/variant.m3u8".to_string())
        );
    }

    #[test]
    fn first_stream_inf_uri_returns_none_for_leaf_playlist() {
        let media = "#EXTM3U\n#EXTINF:6.0,\nseg0.ts\n#EXT-X-ENDLIST\n";
        assert_eq!(
            first_stream_inf_uri(media, "https://example.com/hls/media.m3u8"),
            None
        );
    }

    #[tokio::test]
    async fn download_and_rewrite_media_playlist_rejects_playlist_with_no_segments() {
        let dir = tempfile::tempdir().unwrap();
        let empty_media = "#EXTM3U\n#EXT-X-ENDLIST\n";
        let result = download_and_rewrite_media_playlist(
            empty_media,
            "https://example.com/hls/media.m3u8",
            dir.path(),
        )
        .await;
        assert!(result.is_err());
    }
}
