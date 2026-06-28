use object_store::{
    Attribute, AttributeValue, Attributes, ObjectStore, PutOptions, aws::AmazonS3Builder,
    path::Path as ObjPath,
};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("fetch failed: {0}")]
    FetchFailed(String),
    #[error("upload failed: {0}")]
    UploadFailed(String),
}

#[derive(Clone)]
pub struct StorageService {
    store: Arc<dyn ObjectStore>,
    cdn_base_url: String,
}

impl StorageService {
    pub fn new(
        bucket_name: &str,
        endpoint: &str,
        key_id: &str,
        app_key: &str,
        cdn_base_url: &str,
    ) -> anyhow::Result<Self> {
        // B2 region is the middle segment of the endpoint hostname:
        // s3.us-west-004.backblazeb2.com → us-west-004
        let region = endpoint
            .trim_end_matches(".backblazeb2.com")
            .strip_prefix("s3.")
            .unwrap_or("us-west-004")
            .to_string();
        let store = AmazonS3Builder::new()
            .with_bucket_name(bucket_name)
            .with_endpoint(format!("https://{}", endpoint))
            .with_region(region)
            .with_access_key_id(key_id)
            .with_secret_access_key(app_key)
            // B2 S3-compatible endpoint requires path-style addressing
            .with_virtual_hosted_style_request(false)
            .build()?;
        Ok(Self {
            store: Arc::new(store),
            cdn_base_url: cdn_base_url.trim_end_matches('/').to_string(),
        })
    }

    pub fn is_discord_cdn(url: &str) -> bool {
        url.contains("cdn.discordapp.com") || url.contains("media.discordapp.net")
    }

    pub async fn upload_from_url(&self, url: &str) -> Result<String, StorageError> {
        // Fetch source with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| StorageError::FetchFailed(e.to_string()))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| StorageError::FetchFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(StorageError::FetchFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response
            .bytes()
            .await
            .map_err(|e| StorageError::FetchFailed(e.to_string()))?;

        // Determine output format and convert if needed.
        // WebP conversion is CPU-bound (decode + encode), so we offload it to a
        // blocking thread to avoid tying up a tokio worker.
        let url_for_log = url.to_string();
        let (final_bytes, ext) = if content_type.contains("image/png")
            || content_type.contains("image/jpeg")
        {
            let bytes_clone = bytes.clone();
            match tokio::task::spawn_blocking(move || convert_to_webp(&bytes_clone)).await {
                Ok(Ok(webp_bytes)) => (webp_bytes, "webp"),
                Ok(Err(e)) => {
                    tracing::warn!(
                        "WebP conversion failed for {}, storing original format: {e}",
                        url_for_log
                    );
                    (bytes.to_vec(), ext_from_content_type(&content_type))
                }
                Err(e) => {
                    tracing::warn!(
                        "WebP conversion task panicked for {}, storing original format: {e}",
                        url_for_log
                    );
                    (bytes.to_vec(), ext_from_content_type(&content_type))
                }
            }
        } else if content_type.contains("image/gif") {
            let bytes_clone = bytes.clone();
            match tokio::task::spawn_blocking(move || convert_gif_to_animated_webp(&bytes_clone))
                .await
            {
                Ok(Ok(webp_bytes)) => (webp_bytes, "webp"),
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Animated WebP conversion failed for {}, storing original GIF: {e}",
                        url_for_log
                    );
                    (bytes.to_vec(), "gif")
                }
                Err(e) => {
                    tracing::warn!(
                        "Animated WebP conversion task panicked for {}, storing original GIF: {e}",
                        url_for_log
                    );
                    (bytes.to_vec(), "gif")
                }
            }
        } else if content_type.contains("video/mp4") || content_type.contains("video/webm") {
            match super::video_converter::convert_video_bytes_to_webp(&bytes).await {
                Ok(webp_bytes) => (webp_bytes, "webp"),
                Err(e) => {
                    tracing::warn!(
                        "Video→WebP conversion failed for {}, storing original: {e}",
                        url_for_log
                    );
                    (bytes.to_vec(), ext_from_content_type(&content_type))
                }
            }
        } else {
            (bytes.to_vec(), ext_from_content_type(&content_type))
        };

        // Deterministic key: sha256 of original URL + extension
        let hash = hex::encode(Sha256::digest(url.as_bytes()));
        let key = ObjPath::from(format!("{}.{}", hash, ext));

        tracing::debug!(
            "B2 upload attempt: key={key}, size={} bytes, content_type={}",
            final_bytes.len(),
            mime_for_ext(ext)
        );

        // Set Content-Type so B2 serves the correct MIME type and browsers
        // render <img>/<video> elements without receiving octet-stream.
        let attributes = Attributes::from_iter([(
            Attribute::ContentType,
            AttributeValue::from(mime_for_ext(ext)),
        )]);
        let result = self
            .store
            .put_opts(
                &key,
                final_bytes.into(),
                PutOptions {
                    attributes,
                    ..Default::default()
                },
            )
            .await;

        match &result {
            Ok(_) => tracing::debug!("B2 upload succeeded: key={key}"),
            Err(e) => tracing::error!("B2 upload error (put_opts): {e}"),
        }

        result.map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        Ok(format!("{}/{}", self.cdn_base_url, key))
    }

    /// Upload raw bytes to B2. The object key is derived deterministically
    /// from `source_url` so repeated calls with the same URL are idempotent.
    pub async fn upload_bytes(
        &self,
        source_url: &str,
        bytes: Vec<u8>,
        ext: &str,
    ) -> Result<String, StorageError> {
        let hash = hex::encode(Sha256::digest(source_url.as_bytes()));
        let key = ObjPath::from(format!("{}.{}", hash, ext));

        let attributes = Attributes::from_iter([(
            Attribute::ContentType,
            AttributeValue::from(mime_for_ext(ext)),
        )]);
        self.store
            .put_opts(
                &key,
                bytes.into(),
                PutOptions {
                    attributes,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        Ok(format!("{}/{}", self.cdn_base_url, key))
    }
}

fn convert_to_webp(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let img = image::load_from_memory(bytes)?;
    let mut output = Vec::new();
    img.write_to(&mut Cursor::new(&mut output), image::ImageFormat::WebP)?;
    Ok(output)
}

fn convert_gif_to_animated_webp(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    use image::AnimationDecoder;
    use image::codecs::gif::GifDecoder;

    let decoder = GifDecoder::new(Cursor::new(bytes))?;
    let frames = decoder.into_frames().collect_frames()?;

    if frames.is_empty() {
        anyhow::bail!("GIF has no frames");
    }

    let (width, height) = frames[0].buffer().dimensions();

    // Collect (rgba_bytes, timestamp_ms) before creating the encoder so
    // the data outlives the AnimFrame borrows stored inside AnimEncoder.
    let mut frame_data: Vec<(Vec<u8>, i32)> = Vec::with_capacity(frames.len());
    let mut timestamp_ms: i32 = 0;
    for frame in &frames {
        let (numer, denom) = frame.delay().numer_denom_ms();
        let frame_ms = numer.checked_div(denom).unwrap_or(100) as i32;
        frame_data.push((frame.buffer().as_raw().clone(), timestamp_ms));
        timestamp_ms += frame_ms;
    }

    let config =
        webp::WebPConfig::new().map_err(|_| anyhow::anyhow!("Failed to initialize WebPConfig"))?;
    let mut encoder = webp::AnimEncoder::new(width, height, &config);

    for (rgba, ts) in &frame_data {
        encoder.add_frame(webp::AnimFrame::from_rgba(rgba, width, height, *ts));
    }

    let webp_mem = encoder
        .try_encode()
        .map_err(|e| anyhow::anyhow!("Animated WebP encode failed: {:?}", e))?;
    Ok(webp_mem.to_vec())
}

fn ext_from_content_type(ct: &str) -> &'static str {
    if ct.contains("image/gif") {
        "gif"
    } else if ct.contains("image/webp") {
        "webp"
    } else if ct.contains("image/png") {
        "png"
    } else if ct.contains("image/jpeg") {
        "jpg"
    } else if ct.contains("video/mp4") {
        "mp4"
    } else if ct.contains("video/webm") {
        "webm"
    } else {
        "bin"
    }
}

/// Maps a file extension to the appropriate MIME Content-Type string.
/// Used when uploading to B2 so browsers receive the correct type for
/// <img> and <video> rendering rather than `application/octet-stream`.
fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "webp" => "image/webp",
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_discord_cdn_detects_both_hosts() {
        assert!(StorageService::is_discord_cdn(
            "https://cdn.discordapp.com/attachments/123/456/image.png"
        ));
        assert!(StorageService::is_discord_cdn(
            "https://media.discordapp.net/attachments/123/456/image.gif"
        ));
        assert!(!StorageService::is_discord_cdn(
            "https://example.com/image.png"
        ));
        assert!(!StorageService::is_discord_cdn(
            "https://media.memebucket.app/abc123.webp"
        ));
    }

    #[test]
    fn ext_from_content_type_maps_known_types() {
        assert_eq!(ext_from_content_type("image/gif"), "gif");
        assert_eq!(ext_from_content_type("image/png"), "png");
        assert_eq!(ext_from_content_type("image/jpeg"), "jpg");
        assert_eq!(ext_from_content_type("video/mp4"), "mp4");
        assert_eq!(ext_from_content_type("application/octet-stream"), "bin");
    }

    #[test]
    fn mime_for_ext_maps_all_supported_extensions() {
        assert_eq!(mime_for_ext("webp"), "image/webp");
        assert_eq!(mime_for_ext("png"), "image/png");
        assert_eq!(mime_for_ext("jpg"), "image/jpeg");
        assert_eq!(mime_for_ext("gif"), "image/gif");
        assert_eq!(mime_for_ext("mp4"), "video/mp4");
        assert_eq!(mime_for_ext("webm"), "video/webm");
        assert_eq!(mime_for_ext("bin"), "application/octet-stream");
        assert_eq!(mime_for_ext("unknown"), "application/octet-stream");
    }
}
