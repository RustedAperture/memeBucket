use url::Url;

const METADATA_READ_LIMIT_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageUrlValidationError {
    InvalidHttpUrl,
    FetchFailed,
    UnsupportedContentType,
}

impl ImageUrlValidationError {
    pub fn user_message(self) -> &'static str {
        match self {
            ImageUrlValidationError::InvalidHttpUrl => "URL must be a valid http or https URL.",
            ImageUrlValidationError::FetchFailed => "Image URL could not be fetched.",
            ImageUrlValidationError::UnsupportedContentType => {
                "URL must point to an image or a page with image metadata."
            }
        }
    }
}

pub fn validate_http_url(value: &str) -> bool {
    if value.trim() != value {
        return false;
    }

    Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}

pub async fn validate_image_url(value: &str) -> Result<(), ImageUrlValidationError> {
    if !validate_http_url(value) {
        return Err(ImageUrlValidationError::InvalidHttpUrl);
    }

    let client = image_client()?;
    validate_image_url_with_client(&client, value).await
}

pub async fn resolve_image_url(value: &str) -> Result<String, ImageUrlValidationError> {
    if !validate_http_url(value) {
        return Err(ImageUrlValidationError::InvalidHttpUrl);
    }

    let client = image_client()?;
    if validate_image_url_with_client(&client, value).await.is_ok() {
        return Ok(value.to_string());
    }

    let response = fetch_success(&client, value).await?;
    let Some(content_type) = response_content_type(&response) else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    if !content_type.eq_ignore_ascii_case("text/html") {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    }

    let html = read_limited_text(response).await?;

    if let Some(oembed_url) = find_oembed_url(value, &html)
        && let Some(media_url) = resolve_oembed_photo_url(&client, &oembed_url).await?
    {
        return Ok(media_url);
    }

    for candidate in find_page_image_candidates(value, &html) {
        if validate_image_url_with_client(&client, &candidate)
            .await
            .is_ok()
        {
            return Ok(candidate);
        }
    }

    Err(ImageUrlValidationError::UnsupportedContentType)
}

fn image_client() -> Result<reqwest::Client, ImageUrlValidationError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| ImageUrlValidationError::FetchFailed)
}

async fn validate_image_url_with_client(
    client: &reqwest::Client,
    value: &str,
) -> Result<(), ImageUrlValidationError> {
    let response = fetch_success(client, value).await?;

    let Some(content_type) = response_content_type(&response) else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    if content_type
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("image/"))
    {
        return Ok(());
    }

    Err(ImageUrlValidationError::UnsupportedContentType)
}

async fn fetch_success(
    client: &reqwest::Client,
    value: &str,
) -> Result<reqwest::Response, ImageUrlValidationError> {
    let response = client
        .get(value)
        .send()
        .await
        .map_err(|_| ImageUrlValidationError::FetchFailed)?;

    if !response.status().is_success() {
        return Err(ImageUrlValidationError::FetchFailed);
    }

    Ok(response)
}

fn response_content_type(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
        .into()
}

async fn read_limited_text(
    mut response: reqwest::Response,
) -> Result<String, ImageUrlValidationError> {
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ImageUrlValidationError::FetchFailed)?
    {
        if body.len() + chunk.len() > METADATA_READ_LIMIT_BYTES {
            return Err(ImageUrlValidationError::FetchFailed);
        }
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).map_err(|_| ImageUrlValidationError::UnsupportedContentType)
}

async fn resolve_oembed_photo_url(
    client: &reqwest::Client,
    oembed_url: &str,
) -> Result<Option<String>, ImageUrlValidationError> {
    #[derive(serde::Deserialize)]
    struct OembedResponse {
        #[serde(rename = "type")]
        kind: Option<String>,
        url: Option<String>,
    }

    let response = fetch_success(client, oembed_url).await?;
    let oembed: OembedResponse = response
        .json()
        .await
        .map_err(|_| ImageUrlValidationError::FetchFailed)?;

    let Some(kind) = oembed.kind else {
        return Ok(None);
    };
    if !kind.eq_ignore_ascii_case("photo") {
        return Ok(None);
    }

    let Some(url) = oembed.url else {
        return Ok(None);
    };
    if validate_image_url_with_client(client, &url).await.is_ok() {
        return Ok(Some(url));
    }

    Ok(None)
}

fn find_oembed_url(page_url: &str, html: &str) -> Option<String> {
    find_start_tags(html, "link")
        .into_iter()
        .find(|tag| {
            extract_attr(tag, "type")
                .is_some_and(|value| value.eq_ignore_ascii_case("application/json+oembed"))
        })
        .and_then(|tag| extract_attr(tag, "href"))
        .and_then(|href| absolutize_url(page_url, &href))
}

fn find_page_image_candidates(page_url: &str, html: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for tag in find_start_tags(html, "meta") {
        let key = extract_attr(tag, "property").or_else(|| extract_attr(tag, "name"));
        let Some(key) = key else {
            continue;
        };

        if matches!(
            key.to_ascii_lowercase().as_str(),
            "og:image" | "og:image:url" | "og:image:secure_url" | "twitter:image"
        ) && let Some(content) = extract_attr(tag, "content")
            && let Some(url) = absolutize_url(page_url, &content)
        {
            candidates.push(url);
        }
    }

    for tag in find_start_tags(html, "link") {
        if extract_attr(tag, "rel").is_some_and(|value| {
            value
                .split_ascii_whitespace()
                .any(|part| part.eq_ignore_ascii_case("image_src"))
        }) && let Some(href) = extract_attr(tag, "href")
            && let Some(url) = absolutize_url(page_url, &href)
        {
            candidates.push(url);
        }
    }

    candidates
}

fn find_start_tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let needle = name.to_ascii_lowercase();

    for segment in html.split('<').skip(1) {
        let Some(tag) = segment.split('>').next() else {
            continue;
        };

        let trimmed = tag.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower == needle || lower.starts_with(&(needle.clone() + " ")) {
            tags.push(trimmed);
        }
    }

    tags
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let mut rest = tag;
    loop {
        let index = find_ascii_case_insensitive(rest, attr)?;
        let before = rest[..index].chars().next_back();
        let after = rest[index + attr.len()..].chars().next();

        if before.is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
            || after.is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        {
            rest = &rest[index + attr.len()..];
            continue;
        }

        let mut value = rest[index + attr.len()..].trim_start();
        if !value.starts_with('=') {
            rest = &rest[index + attr.len()..];
            continue;
        }
        value = value[1..].trim_start();

        let Some(quote) = value.chars().next() else {
            return None;
        };

        if quote == '"' || quote == '\'' {
            let value = &value[quote.len_utf8()..];
            let end = value.find(quote)?;
            return Some(decode_html_attr(&value[..end]));
        }

        let end = value.find(char::is_whitespace).unwrap_or(value.len());
        return Some(decode_html_attr(&value[..end]));
    }
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn decode_html_attr(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn absolutize_url(base: &str, value: &str) -> Option<String> {
    let parsed = Url::parse(value)
        .or_else(|_| Url::parse(base)?.join(value))
        .ok()?;
    if matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some() {
        return Some(parsed.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        http::header,
        response::{IntoResponse, Response},
        routing::get,
    };
    use tokio::net::TcpListener;

    async fn spawn_content_type_server(content_type: &'static str) -> String {
        async fn handler(content_type: &'static str) -> Response {
            ([(header::CONTENT_TYPE, content_type)], "body").into_response()
        }

        let app = Router::new().route("/", get(move || handler(content_type)));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        format!("http://{address}/")
    }

    #[tokio::test]
    async fn validate_image_url_accepts_image_content_type() {
        let url = spawn_content_type_server("image/gif").await;

        let result = validate_image_url(&url).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_image_url_rejects_non_image_content_type() {
        let url = spawn_content_type_server("text/html; charset=utf-8").await;

        let result = validate_image_url(&url).await;

        assert!(matches!(
            result,
            Err(ImageUrlValidationError::UnsupportedContentType)
        ));
    }

    #[tokio::test]
    async fn resolve_image_url_keeps_direct_image_urls() {
        let url = spawn_content_type_server("image/gif").await;

        let resolved = resolve_image_url(&url).await.unwrap();

        assert_eq!(resolved, url);
    }

    #[tokio::test]
    async fn resolve_image_url_uses_discovered_oembed_photo_url() {
        async fn page(address: String) -> Response {
            (
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                format!(
                    r#"<html><head><link rel="alternate" type="application/json+oembed" href="http://{address}/oembed"></head></html>"#
                ),
            )
                .into_response()
        }

        async fn oembed(address: String) -> Response {
            (
                [(header::CONTENT_TYPE, "application/json")],
                format!(r#"{{"type":"photo","version":"1.0","url":"http://{address}/image.gif"}}"#),
            )
                .into_response()
        }

        async fn image() -> Response {
            ([(header::CONTENT_TYPE, "image/gif")], "gif").into_response()
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = Router::new()
            .route(
                "/",
                get({
                    let address = address.clone();
                    move || page(address)
                }),
            )
            .route(
                "/oembed",
                get({
                    let address = address.clone();
                    move || oembed(address)
                }),
            )
            .route("/image.gif", get(image));

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let resolved = resolve_image_url(&format!("http://{address}/"))
            .await
            .unwrap();

        assert_eq!(resolved, format!("http://{address}/image.gif"));
    }

    #[tokio::test]
    async fn resolve_image_url_uses_open_graph_image_url() {
        async fn page(address: String) -> Response {
            (
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                format!(
                    r#"<html><head><meta property="og:image" content="http://{address}/image.gif"></head></html>"#
                ),
            )
                .into_response()
        }

        async fn image() -> Response {
            ([(header::CONTENT_TYPE, "image/gif")], "gif").into_response()
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let app = Router::new()
            .route(
                "/",
                get({
                    let address = address.clone();
                    move || page(address)
                }),
            )
            .route("/image.gif", get(image));

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let resolved = resolve_image_url(&format!("http://{address}/"))
            .await
            .unwrap();

        assert_eq!(resolved, format!("http://{address}/image.gif"));
    }
}
