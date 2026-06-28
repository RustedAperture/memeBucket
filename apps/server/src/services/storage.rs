use object_store::{ObjectStore, aws::AmazonS3Builder, path::Path as ObjPath};
use sha2::{Digest, Sha256};
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
        let store = AmazonS3Builder::new()
            .with_bucket_name(bucket_name)
            .with_endpoint(format!("https://{}", endpoint))
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

        // Determine output format and convert if needed
        let (final_bytes, ext) =
            if content_type.contains("image/png") || content_type.contains("image/jpeg") {
                match convert_to_webp(&bytes) {
                    Ok(webp_bytes) => (webp_bytes, "webp"),
                    Err(_) => (bytes.to_vec(), ext_from_content_type(&content_type)),
                }
            } else {
                (bytes.to_vec(), ext_from_content_type(&content_type))
            };

        // Deterministic key: sha256 of original URL + extension
        let hash = hex::encode(Sha256::digest(url.as_bytes()));
        let key = ObjPath::from(format!("{}.{}", hash, ext));

        self.store
            .put(&key, final_bytes.into())
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        Ok(format!("{}/{}", self.cdn_base_url, key))
    }
}

fn convert_to_webp(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let img = image::load_from_memory(bytes)?;
    let mut output = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut output),
        image::ImageFormat::WebP,
    )?;
    Ok(output)
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
}
