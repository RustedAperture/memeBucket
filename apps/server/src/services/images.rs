use url::Url;

pub fn validate_http_url(value: &str) -> bool {
    if value.trim() != value {
        return false;
    }

    Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}
