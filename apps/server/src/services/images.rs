use ipnet::IpNet;
use std::net::IpAddr;
use std::str::FromStr;
use tracing::error;
use url::Url;

const METADATA_READ_LIMIT_BYTES: usize = 512 * 1024;
const BLUESKY_API_BASE_URL: &str = "https://public.api.bsky.app/xrpc";
#[cfg_attr(not(test), allow(dead_code))]
const MAX_HASHTAG_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageUrlValidationError {
    InvalidHttpUrl,
    FetchFailed,
    UnsupportedContentType,
    KlipyPageUrlUnsupported,
}

impl ImageUrlValidationError {
    pub fn user_message(self) -> &'static str {
        match self {
            ImageUrlValidationError::InvalidHttpUrl => "URL must be a valid http or https URL.",
            ImageUrlValidationError::FetchFailed => "Image URL could not be fetched.",
            ImageUrlValidationError::UnsupportedContentType => {
                "URL must point to an image or a page with image metadata."
            }
            ImageUrlValidationError::KlipyPageUrlUnsupported => {
                "Klipy gif/sticker/clip page links can't be imported directly — use \"Search GIFs\" instead to add this."
            }
        }
    }
}

/// Klipy blocks server-side fetches of its individual gif/sticker/clip pages
/// (Cloudflare JS challenge, and robots.txt disallows crawling them), so the
/// generic page-scraping fallback below can never see these pages' content.
/// Detected up front to give a clear, actionable error instead of a generic
/// fetch-failure message.
fn is_klipy_content_page_url(url_str: &str) -> bool {
    let Ok(url) = Url::parse(url_str) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    if !matches!(host, "klipy.com" | "www.klipy.com") {
        return false;
    }

    let Some(mut segments) = url.path_segments() else {
        return false;
    };
    matches!(segments.next(), Some("gifs" | "stickers" | "clips")) && segments.next().is_some()
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

    validate_image_url_internal(value).await
}

#[derive(Debug)]
pub struct ResolvedImage {
    pub url: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BlueskyPostRef {
    handle: String,
    rkey: String,
}

pub async fn resolve_image_url(value: &str) -> Result<ResolvedImage, ImageUrlValidationError> {
    if !validate_http_url(value) {
        return Err(ImageUrlValidationError::InvalidHttpUrl);
    }

    if is_klipy_content_page_url(value) {
        return Err(ImageUrlValidationError::KlipyPageUrlUnsupported);
    }

    if let Some(status_id) = extract_twitter_status_id(value) {
        let media = resolve_twitter_status(&status_id).await?;
        return Ok(ResolvedImage {
            url: media.url,
            notes: media.notes,
            tags: media.tags,
        });
    }

    if let Some(post_ref) = parse_bluesky_post_ref(value) {
        let media = resolve_bluesky_post(&post_ref).await?;
        return Ok(ResolvedImage {
            url: media.url,
            notes: media.notes,
            tags: media.tags,
        });
    }

    if validate_image_url_internal(value).await.is_ok() {
        return Ok(ResolvedImage {
            url: value.to_string(),
            notes: None,
            tags: Vec::new(),
        });
    }

    let response = fetch_success(value).await?;
    let Some(content_type) = response_content_type(&response) else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    if !content_type.eq_ignore_ascii_case("text/html") {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    }

    let html = read_limited_text(response).await?;

    if let Some(oembed_url) = find_oembed_url(value, &html)
        && let Some(media_url) = resolve_oembed_photo_url(&oembed_url).await?
    {
        return Ok(ResolvedImage {
            url: normalize_tenor_url(&media_url),
            notes: None,
            tags: Vec::new(),
        });
    }

    for candidate in find_page_image_candidates(value, &html) {
        let candidate_normalized = normalize_tenor_url(&candidate);
        if validate_image_url_internal(&candidate_normalized)
            .await
            .is_ok()
        {
            return Ok(ResolvedImage {
                url: candidate_normalized,
                notes: None,
                tags: Vec::new(),
            });
        }
    }

    Err(ImageUrlValidationError::UnsupportedContentType)
}

fn normalize_tenor_url(url_str: &str) -> String {
    let Ok(mut url) = Url::parse(url_str) else {
        return url_str.to_string();
    };

    let Some(host) = url.host_str() else {
        return url_str.to_string();
    };

    if host.ends_with(".tenor.com") || host == "tenor.com" {
        let mut path = url.path().to_string();
        if let Some(stripped) = path.strip_prefix("/m/") {
            path = stripped.to_string();
        }

        path = path.replace("AAAPo/", "AAAAC/").replace(".mp4", ".gif");

        let _ = url.set_host(Some("media.tenor.com"));
        url.set_path(&path);
        return url.to_string();
    }

    url_str.to_string()
}

fn extract_twitter_status_id(url_str: &str) -> Option<String> {
    let url = Url::parse(url_str).ok()?;
    let host = url.host_str()?;

    if !matches!(host, "x.com" | "twitter.com" | "mobile.twitter.com") {
        return None;
    }

    let segments: Vec<&str> = url.path_segments()?.collect();
    let status_index = segments.iter().position(|segment| *segment == "status")?;
    let id = segments.get(status_index + 1)?;

    if !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() {
        return None;
    }

    Some(id.to_string())
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_bluesky_post_ref(url_str: &str) -> Option<BlueskyPostRef> {
    let url = Url::parse(url_str).ok()?;

    if !matches!(url.host_str()?, "bsky.app" | "bsky.social") {
        return None;
    }

    let mut segments = url.path_segments()?;
    let [profile, handle, post, rkey] = [
        segments.next()?,
        segments.next()?,
        segments.next()?,
        segments.next()?,
    ];

    if profile != "profile"
        || post != "post"
        || handle.is_empty()
        || rkey.is_empty()
        || segments.next().is_some()
    {
        return None;
    }

    Some(BlueskyPostRef {
        handle: handle.to_string(),
        rkey: rkey.to_string(),
    })
}

#[cfg_attr(not(test), allow(dead_code))]
fn extract_hashtags_from_text(text: &str) -> Vec<String> {
    let mut hashtags = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] != '#' {
            index += 1;
            continue;
        }

        let mut end = index + 1;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if end > index + 1 {
            let hashtag: String = chars[index + 1..end].iter().collect();
            push_hashtag(&mut hashtags, &hashtag);
        }

        index = end;
    }

    hashtags
}

fn push_hashtag(hashtags: &mut Vec<String>, value: &str) {
    let hashtag = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hashtag.is_empty()
        || hashtag.len() > MAX_HASHTAG_LEN
        || !hashtag
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        || hashtags
            .iter()
            .any(|existing| hashtags_equal_case_insensitively(existing, hashtag))
    {
        return;
    }

    hashtags.push(hashtag.to_string());
}

fn hashtags_equal_case_insensitively(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_uppercase)
        .eq(right.chars().flat_map(char::to_uppercase))
}

#[derive(serde::Deserialize)]
struct BlueskyHandleResponse {
    did: String,
}

#[derive(serde::Deserialize)]
struct BlueskyPostsResponse {
    posts: Vec<BlueskyPost>,
}

#[derive(serde::Deserialize)]
struct BlueskyPost {
    author: BlueskyAuthor,
    record: BlueskyRecord,
    embed: Option<BlueskyEmbed>,
}

#[derive(serde::Deserialize)]
struct BlueskyAuthor {
    handle: String,
}

#[derive(serde::Deserialize)]
struct BlueskyRecord {
    #[serde(default)]
    text: String,
    #[serde(default)]
    facets: Vec<BlueskyFacet>,
}

#[derive(serde::Deserialize)]
struct BlueskyFacet {
    #[serde(default)]
    features: Vec<BlueskyFacetFeature>,
}

#[derive(serde::Deserialize)]
struct BlueskyFacetFeature {
    #[serde(rename = "$type")]
    kind: Option<String>,
    tag: Option<String>,
}

#[derive(serde::Deserialize)]
struct BlueskyEmbed {
    #[serde(rename = "$type")]
    kind: String,
    #[serde(default)]
    images: Vec<BlueskyImage>,
    playlist: Option<String>,
}

#[derive(serde::Deserialize)]
struct BlueskyImage {
    fullsize: Option<String>,
}

struct BlueskyMedia {
    url: String,
    notes: Option<String>,
    tags: Vec<String>,
}

fn bluesky_handle_api_url(handle: &str) -> String {
    let mut url = Url::parse(&format!(
        "{BLUESKY_API_BASE_URL}/com.atproto.identity.resolveHandle"
    ))
    .expect("Bluesky handle endpoint must be a valid URL");
    url.query_pairs_mut().append_pair("handle", handle);
    url.to_string()
}

fn bluesky_post_api_url_from_endpoint(
    endpoint: &str,
    did: &str,
    rkey: &str,
) -> Result<String, ImageUrlValidationError> {
    let mut url = Url::parse(endpoint).map_err(|_| ImageUrlValidationError::FetchFailed)?;
    let uri = format!("at://{did}/app.bsky.feed.post/{rkey}");
    {
        let mut query = url.query_pairs_mut();
        query.clear();
        query.append_pair("uris", &uri);
    }
    Ok(url.to_string())
}

async fn resolve_bluesky_post(
    post_ref: &BlueskyPostRef,
) -> Result<BlueskyMedia, ImageUrlValidationError> {
    let handle_url = bluesky_handle_api_url(&post_ref.handle);
    let post_endpoint = format!("{BLUESKY_API_BASE_URL}/app.bsky.feed.getPosts");
    resolve_bluesky_post_from_api_urls(
        &post_ref.handle,
        &post_ref.rkey,
        &handle_url,
        &post_endpoint,
    )
    .await
}

async fn resolve_bluesky_post_from_api_urls(
    _handle: &str,
    _rkey: &str,
    handle_url: &str,
    post_url: &str,
) -> Result<BlueskyMedia, ImageUrlValidationError> {
    let handle_response = fetch_success(handle_url).await?;
    let handle_body = read_limited_text(handle_response).await?;
    let handle: BlueskyHandleResponse =
        serde_json::from_str(&handle_body).map_err(|_| ImageUrlValidationError::FetchFailed)?;

    if handle.did.trim().is_empty() {
        return Err(ImageUrlValidationError::FetchFailed);
    }

    let post_url = bluesky_post_api_url_from_endpoint(post_url, &handle.did, _rkey)?;
    let response = fetch_success(&post_url).await?;
    let body = read_limited_text(response).await?;
    let response: BlueskyPostsResponse =
        serde_json::from_str(&body).map_err(|_| ImageUrlValidationError::FetchFailed)?;
    let post = response
        .posts
        .into_iter()
        .next()
        .ok_or(ImageUrlValidationError::UnsupportedContentType)?;
    if post.author.handle.trim().is_empty() {
        return Err(ImageUrlValidationError::FetchFailed);
    }
    let embed = post
        .embed
        .ok_or(ImageUrlValidationError::UnsupportedContentType)?;

    let url = match embed.kind.as_str() {
        "app.bsky.embed.images#view" => embed
            .images
            .into_iter()
            .find_map(|image| image.fullsize.filter(|url| !url.trim().is_empty()))
            .ok_or(ImageUrlValidationError::UnsupportedContentType)?,
        "app.bsky.embed.video#view" => embed
            .playlist
            .filter(|url| !url.trim().is_empty())
            .ok_or(ImageUrlValidationError::UnsupportedContentType)?,
        _ => return Err(ImageUrlValidationError::UnsupportedContentType),
    };

    validate_bluesky_media_url(&url).await?;

    let mut tags = Vec::new();
    for feature in post
        .record
        .facets
        .into_iter()
        .flat_map(|facet| facet.features)
    {
        if feature
            .kind
            .as_deref()
            .is_some_and(|kind| kind == "app.bsky.richtext.facet#tag")
            && let Some(tag) = feature.tag
        {
            push_hashtag(&mut tags, &tag);
        }
    }
    if tags.is_empty() {
        tags = extract_hashtags_from_text(&post.record.text);
    }

    Ok(BlueskyMedia {
        url,
        notes: Some(format_twitter_notes(&post.author.handle, &post.record.text)),
        tags,
    })
}

async fn validate_bluesky_media_url(value: &str) -> Result<(), ImageUrlValidationError> {
    if !validate_http_url(value) {
        return Err(ImageUrlValidationError::InvalidHttpUrl);
    }

    let response = fetch_success(value).await?;
    let Some(content_type) = response_content_type(&response) else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    if content_type.get(..6).is_some_and(|prefix| {
        prefix.eq_ignore_ascii_case("image/") || prefix.eq_ignore_ascii_case("video/")
    }) || matches!(
        content_type.to_ascii_lowercase().as_str(),
        "application/vnd.apple.mpegurl" | "application/x-mpegurl"
    ) {
        return Ok(());
    }

    Err(ImageUrlValidationError::UnsupportedContentType)
}

fn is_safe_ip(ip: &IpAddr) -> bool {
    #[cfg(test)]
    if ip.is_loopback() {
        return true;
    }

    if ip.is_loopback() && std::env::var("MEMEBUCKET_ALLOW_LOCAL_IPS_IN_TESTS").is_ok() {
        return true;
    }

    let loopback = IpNet::from_str("127.0.0.0/8").unwrap();
    let private_10 = IpNet::from_str("10.0.0.0/8").unwrap();
    let private_172 = IpNet::from_str("172.16.0.0/12").unwrap();
    let private_192 = IpNet::from_str("192.168.0.0/16").unwrap();
    let aws_metadata = IpNet::from_str("169.254.169.254/32").unwrap();

    if loopback.contains(ip)
        || private_10.contains(ip)
        || private_172.contains(ip)
        || private_192.contains(ip)
        || aws_metadata.contains(ip)
    {
        return false;
    }

    match ip {
        IpAddr::V4(ipv4) => !ipv4.is_private() && !ipv4.is_loopback() && !ipv4.is_link_local(),
        IpAddr::V6(ipv6) => !ipv6.is_loopback(),
    }
}

async fn validate_image_url_internal(value: &str) -> Result<(), ImageUrlValidationError> {
    let response = fetch_success(value).await?;

    let Some(content_type) = response_content_type(&response) else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    if content_type.get(..6).is_some_and(|prefix| {
        prefix.eq_ignore_ascii_case("image/") || prefix.eq_ignore_ascii_case("video/")
    }) {
        return Ok(());
    }

    Err(ImageUrlValidationError::UnsupportedContentType)
}

pub(crate) async fn fetch_success(
    value: &str,
) -> Result<reqwest::Response, ImageUrlValidationError> {
    let mut current_url_str = value.to_string();
    let mut redirects = 0;

    loop {
        if redirects > 5 {
            return Err(ImageUrlValidationError::FetchFailed);
        }
        let parsed_url =
            Url::parse(&current_url_str).map_err(|_| ImageUrlValidationError::InvalidHttpUrl)?;
        let host = parsed_url
            .host_str()
            .ok_or(ImageUrlValidationError::InvalidHttpUrl)?;
        let port =
            parsed_url
                .port_or_known_default()
                .unwrap_or(if parsed_url.scheme() == "https" {
                    443
                } else {
                    80
                });

        let mut addrs = tokio::net::lookup_host(format!("{}:{}", host, port))
            .await
            .map_err(|e| {
                error!("DNS lookup failed for {}: {}", host, e);
                ImageUrlValidationError::FetchFailed
            })?;

        let safe_addr = addrs.find(|addr| is_safe_ip(&addr.ip())).ok_or_else(|| {
            error!("No safe IP found for {}", host);
            ImageUrlValidationError::FetchFailed
        })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 memeBucketBot/1.0")
            .resolve(host, safe_addr)
            .build()
            .map_err(|e| {
                error!("Failed to build reqwest client: {}", e);
                ImageUrlValidationError::FetchFailed
            })?;

        let response = client.get(&current_url_str).send().await.map_err(|e| {
            error!("Failed to send request to {}: {}", current_url_str, e);
            ImageUrlValidationError::FetchFailed
        })?;

        if response.status().is_redirection() {
            let loc = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or_else(|| {
                    error!(
                        "Missing LOCATION header in redirect for {}",
                        current_url_str
                    );
                    ImageUrlValidationError::FetchFailed
                })?;
            let loc_str = loc.to_str().map_err(|e| {
                error!("Invalid LOCATION header: {}", e);
                ImageUrlValidationError::FetchFailed
            })?;
            let next_url = parsed_url.join(loc_str).map_err(|e| {
                error!("Invalid redirect URL {}: {}", loc_str, e);
                ImageUrlValidationError::FetchFailed
            })?;
            if !matches!(next_url.scheme(), "http" | "https") {
                error!("Invalid redirect scheme: {}", next_url.scheme());
                return Err(ImageUrlValidationError::FetchFailed);
            }
            current_url_str = next_url.to_string();
            redirects += 1;
            continue;
        }

        if !response.status().is_success() {
            error!(
                "Request to {} failed with status {}",
                current_url_str,
                response.status()
            );
            return Err(ImageUrlValidationError::FetchFailed);
        }

        return Ok(response);
    }
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

async fn read_limited_text(response: reqwest::Response) -> Result<String, ImageUrlValidationError> {
    let body = read_limited_bytes(response, METADATA_READ_LIMIT_BYTES).await?;
    String::from_utf8(body).map_err(|_| ImageUrlValidationError::UnsupportedContentType)
}

/// Reads a response body up to `limit_bytes`, rejecting anything larger.
/// Used for both HTML/JSON metadata scraping and safe-fetched HLS assets.
pub(crate) async fn read_limited_bytes(
    mut response: reqwest::Response,
    limit_bytes: usize,
) -> Result<Vec<u8>, ImageUrlValidationError> {
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ImageUrlValidationError::FetchFailed)?
    {
        if body.len() + chunk.len() > limit_bytes {
            return Err(ImageUrlValidationError::FetchFailed);
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

async fn resolve_oembed_photo_url(
    oembed_url: &str,
) -> Result<Option<String>, ImageUrlValidationError> {
    #[derive(serde::Deserialize)]
    struct OembedResponse {
        #[serde(rename = "type")]
        kind: Option<String>,
        url: Option<String>,
    }

    let response = fetch_success(oembed_url).await?;
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
    if validate_image_url_internal(&url).await.is_ok() {
        return Ok(Some(url));
    }

    Ok(None)
}

#[derive(serde::Deserialize)]
struct SyndicationPhoto {
    url: String,
}

#[derive(serde::Deserialize)]
struct SyndicationVideoVariant {
    bitrate: Option<u64>,
    src: String,
}

#[derive(serde::Deserialize)]
struct SyndicationVideo {
    variants: Vec<SyndicationVideoVariant>,
}

#[derive(serde::Deserialize)]
struct SyndicationUser {
    screen_name: String,
}

#[derive(serde::Deserialize)]
struct SyndicationResponse {
    photos: Vec<SyndicationPhoto>,
    video: Option<SyndicationVideo>,
    text: Option<String>,
    user: Option<SyndicationUser>,
}

struct TwitterMedia {
    url: String,
    notes: Option<String>,
    tags: Vec<String>,
}

fn format_twitter_notes(screen_name: &str, text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        format!("@{screen_name}")
    } else {
        format!("@{screen_name}: {trimmed}")
    }
}

fn parse_syndication_response(body: &str) -> Result<TwitterMedia, ImageUrlValidationError> {
    let response: SyndicationResponse =
        serde_json::from_str(body).map_err(|_| ImageUrlValidationError::FetchFailed)?;

    let url = if let Some(photo) = response.photos.first() {
        photo.url.clone()
    } else if let Some(video) = response.video
        && let Some(best) = video
            .variants
            .into_iter()
            .max_by_key(|variant| variant.bitrate.unwrap_or(0))
    {
        best.src
    } else {
        return Err(ImageUrlValidationError::UnsupportedContentType);
    };

    let notes = response.user.map(|user| {
        format_twitter_notes(&user.screen_name, response.text.as_deref().unwrap_or(""))
    });
    let tags = response
        .text
        .as_deref()
        .map(extract_hashtags_from_text)
        .unwrap_or_default();

    Ok(TwitterMedia { url, notes, tags })
}

async fn resolve_twitter_status(id: &str) -> Result<TwitterMedia, ImageUrlValidationError> {
    let api_url = format!(
        "https://cdn.syndication.twimg.com/tweet-result?id={}&lang=en&token=memebucket",
        id
    );

    resolve_twitter_status_from_api_url(&api_url).await
}

/// Split out from `resolve_twitter_status` so tests can point it at a local
/// mock server instead of the real syndication endpoint.
async fn resolve_twitter_status_from_api_url(
    api_url: &str,
) -> Result<TwitterMedia, ImageUrlValidationError> {
    let response = fetch_success(api_url).await?;
    let body = read_limited_text(response).await?;

    parse_syndication_response(&body)
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

        let quote = value.chars().next()?;

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
    use tokio::{net::TcpListener, sync::Mutex};

    // Some sandboxed test environments limit concurrent loopback binds. Serialize only the bind
    // operation so mock servers can continue handling requests concurrently.
    static MOCK_SERVER_LOCK: Mutex<()> = Mutex::const_new(());

    async fn bind_mock_server() -> (TcpListener, std::net::SocketAddr) {
        let _lock = MOCK_SERVER_LOCK.lock().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        (listener, address)
    }

    fn serve_mock_server(listener: TcpListener, app: Router) {
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    async fn spawn_content_type_server(content_type: &'static str) -> String {
        async fn handler(content_type: &'static str) -> Response {
            ([(header::CONTENT_TYPE, content_type)], "body").into_response()
        }

        let app = Router::new().route("/", get(move || handler(content_type)));
        let (listener, address) = bind_mock_server().await;
        serve_mock_server(listener, app);

        format!("http://{address}/")
    }

    async fn spawn_bluesky_api_server(
        post_body: String,
        media_content_type: &'static str,
    ) -> String {
        async fn handle() -> Response {
            (
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"did":"did:plc:alice"}"#,
            )
                .into_response()
        }

        let (listener, address) = bind_mock_server().await;
        let address = address.to_string();
        let post_body = post_body.replace("{address}", &address);
        let app = Router::new()
            .route("/xrpc/com.atproto.identity.resolveHandle", get(handle))
            .route(
                "/xrpc/app.bsky.feed.getPosts",
                get(move || {
                    let post_body = post_body.clone();
                    async move {
                        ([(header::CONTENT_TYPE, "application/json")], post_body).into_response()
                    }
                }),
            )
            .route(
                "/image.gif",
                get(|| async { ([(header::CONTENT_TYPE, "image/gif")], "gif") }),
            )
            .route(
                "/playlist.m3u8",
                get(move || async move { ([(header::CONTENT_TYPE, media_content_type)], "video") }),
            );

        serve_mock_server(listener, app);

        format!("http://{address}")
    }

    fn bluesky_api_urls(base_url: &str) -> (String, String) {
        (
            format!("{base_url}/xrpc/com.atproto.identity.resolveHandle?handle=alice.bsky.social"),
            format!(
                "{base_url}/xrpc/app.bsky.feed.getPosts?uris=at%3A%2F%2Fdid%3Aplc%3Aalice%2Fapp.bsky.feed.post%2Frkey"
            ),
        )
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

        assert_eq!(resolved.url, url);
    }

    #[tokio::test]
    async fn resolve_image_url_ignores_non_twitter_hosts() {
        let url = spawn_content_type_server("image/gif").await;

        // A non-Twitter URL must still go through the generic scraper path,
        // proving the Twitter branch doesn't intercept unrelated hosts.
        let resolved = resolve_image_url(&url).await.unwrap();

        assert_eq!(resolved.url, url);
    }

    #[tokio::test]
    async fn resolve_image_url_returns_none_notes_for_non_twitter_urls() {
        let url = spawn_content_type_server("image/gif").await;

        let resolved = resolve_image_url(&url).await.unwrap();

        assert_eq!(resolved.url, url);
        assert_eq!(resolved.notes, None);
    }

    #[test]
    fn is_klipy_content_page_url_matches_gifs_stickers_and_clips() {
        assert!(is_klipy_content_page_url(
            "https://klipy.com/gifs/cats-kitties-1"
        ));
        assert!(is_klipy_content_page_url(
            "https://www.klipy.com/stickers/some-sticker"
        ));
        assert!(is_klipy_content_page_url(
            "https://klipy.com/clips/some-clip"
        ));
    }

    #[test]
    fn is_klipy_content_page_url_rejects_bare_gifs_root_and_other_hosts() {
        // The bare "/gifs" root (no slug) is the one path klipy.com's
        // robots.txt actually allows crawlers to visit — not a content page.
        assert!(!is_klipy_content_page_url("https://klipy.com/gifs"));
        assert!(!is_klipy_content_page_url("https://klipy.com/developers"));
        // Klipy's own asset CDN serves direct media, not a bot-blocked page.
        assert!(!is_klipy_content_page_url(
            "https://static.klipy.com/ii/some/asset.gif"
        ));
        assert!(!is_klipy_content_page_url(
            "https://example.com/gifs/cats-kitties-1"
        ));
    }

    #[tokio::test]
    async fn resolve_image_url_rejects_klipy_content_page_urls_with_helpful_message() {
        let err = resolve_image_url("https://klipy.com/gifs/cats-kitties-1")
            .await
            .unwrap_err();

        assert_eq!(err, ImageUrlValidationError::KlipyPageUrlUnsupported);
        assert!(err.user_message().contains("Search GIFs"));
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

        let (listener, address) = bind_mock_server().await;
        let address = address.to_string();
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

        serve_mock_server(listener, app);

        let resolved = resolve_image_url(&format!("http://{address}/"))
            .await
            .unwrap();

        assert_eq!(resolved.url, format!("http://{address}/image.gif"));
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

        let (listener, address) = bind_mock_server().await;
        let address = address.to_string();
        let app = Router::new()
            .route(
                "/",
                get({
                    let address = address.clone();
                    move || page(address)
                }),
            )
            .route("/image.gif", get(image));

        serve_mock_server(listener, app);

        let resolved = resolve_image_url(&format!("http://{address}/"))
            .await
            .unwrap();

        assert_eq!(resolved.url, format!("http://{address}/image.gif"));
    }

    #[test]
    fn extract_twitter_status_id_matches_x_com() {
        assert_eq!(
            extract_twitter_status_id(
                "https://x.com/protogenElvis/status/2076683958096646274?s=20"
            ),
            Some("2076683958096646274".to_string())
        );
    }

    #[test]
    fn extract_twitter_status_id_matches_twitter_com() {
        assert_eq!(
            extract_twitter_status_id(
                "https://twitter.com/AriesArtistFIN/status/2076662019177021747"
            ),
            Some("2076662019177021747".to_string())
        );
    }

    #[test]
    fn extract_twitter_status_id_matches_mobile_twitter_com() {
        assert_eq!(
            extract_twitter_status_id("https://mobile.twitter.com/someuser/status/123456"),
            Some("123456".to_string())
        );
    }

    #[test]
    fn extract_twitter_status_id_matches_with_trailing_photo_segment() {
        assert_eq!(
            extract_twitter_status_id("https://x.com/someuser/status/123456/photo/1"),
            Some("123456".to_string())
        );
    }

    #[test]
    fn extract_twitter_status_id_returns_none_for_non_status_path() {
        assert_eq!(extract_twitter_status_id("https://x.com/someuser"), None);
    }

    #[test]
    fn extract_twitter_status_id_returns_none_for_other_hosts() {
        assert_eq!(
            extract_twitter_status_id("https://example.com/someuser/status/123456"),
            None
        );
    }

    #[test]
    fn extract_twitter_status_id_returns_none_for_non_numeric_id() {
        assert_eq!(
            extract_twitter_status_id("https://x.com/someuser/status/not-a-number"),
            None
        );
    }

    #[test]
    fn parse_bluesky_post_ref_matches_bsky_profile_post_url() {
        assert_eq!(
            parse_bluesky_post_ref(
                "https://bsky.app/profile/kuroisuu.bsky.social/post/3mqnaxhwiks2c"
            ),
            Some(BlueskyPostRef {
                handle: "kuroisuu.bsky.social".to_string(),
                rkey: "3mqnaxhwiks2c".to_string(),
            })
        );
    }

    #[test]
    fn parse_bluesky_post_ref_matches_bsky_social_with_query_string() {
        assert_eq!(
            parse_bluesky_post_ref(
                "https://bsky.social/profile/alice.bsky.social/post/3mqnaxhwiks2c?ref=profile"
            ),
            Some(BlueskyPostRef {
                handle: "alice.bsky.social".to_string(),
                rkey: "3mqnaxhwiks2c".to_string(),
            })
        );
    }

    #[test]
    fn parse_bluesky_post_ref_rejects_non_bluesky_hosts() {
        assert_eq!(
            parse_bluesky_post_ref(
                "https://example.com/profile/kuroisuu.bsky.social/post/3mqnaxhwiks2c"
            ),
            None
        );
    }

    #[test]
    fn parse_bluesky_post_ref_rejects_malformed_paths() {
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/profile/kuroisuu.bsky.social"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/profile//post/3mqnaxhwiks2c"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/profile/kuroisuu.bsky.social/post/"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/post/3mqnaxhwiks2c"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref(
                "https://bsky.app/profile/kuroisuu.bsky.social/post/3mqnaxhwiks2c/extra"
            ),
            None
        );
    }

    #[tokio::test]
    async fn resolve_bluesky_post_selects_first_image_and_formats_notes() {
        let base_url = spawn_bluesky_api_server(
            r##"{
                "posts": [{
                    "author": {"handle": "alice.bsky.social"},
                    "record": {"text": "Look at this"},
                    "embed": {
                        "$type": "app.bsky.embed.images#view",
                        "images": [{"fullsize": "http://{address}/image.gif"}]
                    }
                }]
            }"##
            .to_string(),
            "video/mp4",
        )
        .await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let resolved =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await
                .unwrap();

        assert_eq!(resolved.url, format!("{base_url}/image.gif"));
        assert_eq!(
            resolved.notes.as_deref(),
            Some("@alice.bsky.social: Look at this")
        );
        assert!(resolved.tags.is_empty());
    }

    #[tokio::test]
    async fn resolve_bluesky_post_selects_video_playlist() {
        let base_url = spawn_bluesky_api_server(
            r##"{
                "posts": [{
                    "author": {"handle": "alice.bsky.social"},
                    "record": {"text": ""},
                    "embed": {
                        "$type": "app.bsky.embed.video#view",
                        "playlist": "http://{address}/playlist.m3u8"
                    }
                }]
            }"##
            .to_string(),
            "video/mp4",
        )
        .await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let resolved =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await
                .unwrap();

        assert_eq!(resolved.url, format!("{base_url}/playlist.m3u8"));
        assert_eq!(resolved.notes.as_deref(), Some("@alice.bsky.social"));
    }

    #[tokio::test]
    async fn resolve_bluesky_post_prefers_facet_tags_over_text_hashtags() {
        let base_url = spawn_bluesky_api_server(
            r##"{
                "posts": [{
                    "author": {"handle": "alice.bsky.social"},
                    "record": {
                        "text": "#ignored",
                        "facets": [{
                            "features": [
                                {"$type": "app.bsky.richtext.facet#tag", "tag": "First"},
                                {"$type": "app.bsky.richtext.facet#tag", "tag": "second"}
                            ]
                        }]
                    },
                    "embed": {
                        "$type": "app.bsky.embed.images#view",
                        "images": [{"fullsize": "http://{address}/image.gif"}]
                    }
                }]
            }"##
            .to_string(),
            "video/mp4",
        )
        .await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let resolved =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await
                .unwrap();

        assert_eq!(resolved.tags, vec!["First", "second"]);
    }

    #[tokio::test]
    async fn resolve_bluesky_post_falls_back_to_text_hashtags_without_facets() {
        let base_url = spawn_bluesky_api_server(
            r##"{
                "posts": [{
                    "author": {"handle": "alice.bsky.social"},
                    "record": {"text": "#First #second #FIRST"},
                    "embed": {
                        "$type": "app.bsky.embed.images#view",
                        "images": [{"fullsize": "http://{address}/image.gif"}]
                    }
                }]
            }"##
            .to_string(),
            "video/mp4",
        )
        .await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let resolved =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await
                .unwrap();

        assert_eq!(resolved.tags, vec!["First", "second"]);
    }

    #[tokio::test]
    async fn resolve_bluesky_post_rejects_missing_media() {
        let base_url = spawn_bluesky_api_server(
            r#"{"posts":[{"author":{"handle":"alice.bsky.social"},"record":{"text":"hello"}}]}"#
                .to_string(),
            "video/mp4",
        )
        .await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let result =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await;

        assert!(matches!(
            result,
            Err(ImageUrlValidationError::UnsupportedContentType)
        ));
    }

    #[tokio::test]
    async fn resolve_bluesky_post_rejects_malformed_upstream_payload() {
        let base_url = spawn_bluesky_api_server("not json".to_string(), "video/mp4").await;
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let result =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await;

        assert!(matches!(result, Err(ImageUrlValidationError::FetchFailed)));
    }

    #[tokio::test]
    async fn resolve_bluesky_post_rejects_upstream_errors() {
        async fn unavailable() -> axum::http::StatusCode {
            axum::http::StatusCode::BAD_GATEWAY
        }

        let app = Router::new().route("/xrpc/com.atproto.identity.resolveHandle", get(unavailable));
        let (listener, address) = bind_mock_server().await;
        serve_mock_server(listener, app);
        let base_url = format!("http://{address}");
        let (handle_url, post_url) = bluesky_api_urls(&base_url);

        let result =
            resolve_bluesky_post_from_api_urls("alice.bsky.social", "rkey", &handle_url, &post_url)
                .await;

        assert!(matches!(result, Err(ImageUrlValidationError::FetchFailed)));
    }

    #[test]
    fn extract_hashtags_matches_x_text() {
        assert_eq!(
            extract_hashtags_from_text("Look #One #two_three #日本語 #déjàvu and plain text"),
            vec![
                "One".to_string(),
                "two_three".to_string(),
                "日本語".to_string(),
                "déjàvu".to_string(),
            ]
        );
    }

    #[test]
    fn extract_hashtags_deduplicates_case_insensitively_and_skips_invalid_tags() {
        let overlong = format!("#{}", "x".repeat(65));

        assert_eq!(
            extract_hashtags_from_text(&format!("#First #first #FIRST #_ok #! {} #", overlong)),
            vec!["First".to_string(), "_ok".to_string()]
        );
    }

    #[test]
    fn extract_hashtags_deduplicates_unicode_case_insensitively() {
        assert_eq!(
            extract_hashtags_from_text("#Éclair #éclair"),
            vec!["Éclair".to_string()]
        );
    }

    #[test]
    fn parse_syndication_response_includes_text_hashtags() {
        let body = r##"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null,"text":"#First #second #FIRST"}"##;

        let result = parse_syndication_response(body).unwrap();

        assert_eq!(result.tags, vec!["First", "second"]);
    }

    #[test]
    fn normalize_tenor_url_normalizes_media_host_variants() {
        assert_eq!(
            normalize_tenor_url("https://media1.tenor.com/m/AbCd/AAAAC/example.gif?foo=bar"),
            "https://media.tenor.com/AbCd/AAAAC/example.gif?foo=bar"
        );
        assert_eq!(
            normalize_tenor_url("https://media2.tenor.com/m/AbCd/AAAPo/example.mp4"),
            "https://media.tenor.com/AbCd/AAAAC/example.gif"
        );
    }

    #[test]
    fn tenor_page_metadata_candidate_is_selected_and_normalized() {
        let submitted_page_url = "https://tenor.com/view/dancing-cat-123";
        let html = r#"<meta property="og:image" content="https://media1.tenor.com/m/AbCd/AAAAC/example.gif">"#;

        let candidates = find_page_image_candidates(submitted_page_url, html);

        assert_eq!(
            candidates,
            vec!["https://media1.tenor.com/m/AbCd/AAAAC/example.gif"]
        );
        assert_eq!(
            normalize_tenor_url(&candidates[0]),
            "https://media.tenor.com/AbCd/AAAAC/example.gif"
        );
    }

    #[test]
    fn extract_bsky_post_ref_accepts_profile_post_url() {
        assert_eq!(
            parse_bluesky_post_ref(
                "https://bsky.app/profile/kuroisuu.bsky.social/post/3mqnaxhwiks2c"
            ),
            Some(BlueskyPostRef {
                handle: "kuroisuu.bsky.social".to_string(),
                rkey: "3mqnaxhwiks2c".to_string(),
            })
        );
    }

    #[test]
    fn extract_bsky_post_ref_rejects_other_hosts_and_malformed_paths() {
        assert_eq!(
            parse_bluesky_post_ref("https://example.com/profile/user/post/abc"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/profile/user"),
            None
        );
        assert_eq!(
            parse_bluesky_post_ref("https://bsky.app/profile/user/post/abc/extra"),
            None
        );
    }

    #[test]
    fn extract_hashtags_preserves_first_spelling_and_deduplicates_case_insensitively() {
        assert_eq!(
            extract_hashtags_from_text("Look #Funny #funny #猫_and_猫 #ignored #"),
            vec!["Funny", "猫_and_猫", "ignored"]
        );
    }

    #[test]
    fn extract_hashtags_ignores_empty_and_overlong_tags() {
        let overlong = "a".repeat(65);
        assert_eq!(
            extract_hashtags_from_text(&format!("# #{} #ok", overlong)),
            vec!["ok"]
        );
    }

    #[test]
    fn parse_syndication_response_prefers_photos() {
        let body =
            r#"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.url, "https://pbs.twimg.com/media/abc.jpg:large");
    }

    #[test]
    fn parse_syndication_response_picks_highest_bitrate_video_variant() {
        let body = r#"{"photos":[],"video":{"variants":[
            {"type":"video/mp4","bitrate":832000,"src":"https://video.twimg.com/med.mp4"},
            {"type":"video/mp4","bitrate":2176000,"src":"https://video.twimg.com/high.mp4"},
            {"type":"video/mp4","bitrate":256000,"src":"https://video.twimg.com/low.mp4"}
        ]}}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.url, "https://video.twimg.com/high.mp4");
    }

    #[test]
    fn parse_syndication_response_handles_gif_single_variant() {
        let body = r#"{"photos":[],"video":{"variants":[
            {"type":"video/mp4","bitrate":0,"src":"https://video.twimg.com/tweet_video/abc.mp4"}
        ]}}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.url, "https://video.twimg.com/tweet_video/abc.mp4");
    }

    #[test]
    fn parse_syndication_response_errors_on_no_media() {
        let body = r#"{"photos":[],"video":null}"#;
        let result = parse_syndication_response(body);
        assert!(matches!(
            result,
            Err(ImageUrlValidationError::UnsupportedContentType)
        ));
    }

    #[test]
    fn parse_syndication_response_errors_on_malformed_json() {
        let result = parse_syndication_response("not json");
        assert!(matches!(result, Err(ImageUrlValidationError::FetchFailed)));
    }

    #[test]
    fn format_twitter_notes_includes_handle_and_text() {
        let result = format_twitter_notes("protogenElvis", "You know I've been thinking lately");
        assert_eq!(result, "@protogenElvis: You know I've been thinking lately");
    }

    #[test]
    fn format_twitter_notes_falls_back_to_handle_only_when_text_is_empty() {
        assert_eq!(format_twitter_notes("protogenElvis", ""), "@protogenElvis");
        assert_eq!(
            format_twitter_notes("protogenElvis", "   "),
            "@protogenElvis"
        );
    }

    #[test]
    fn parse_syndication_response_includes_notes_when_user_and_text_present() {
        let body = r#"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null,"text":"You know I've been thinking lately","user":{"screen_name":"protogenElvis"}}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.url, "https://pbs.twimg.com/media/abc.jpg:large");
        assert_eq!(
            result.notes.as_deref(),
            Some("@protogenElvis: You know I've been thinking lately")
        );
    }

    #[test]
    fn parse_syndication_response_notes_falls_back_to_handle_only_when_text_empty() {
        let body = r#"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null,"text":"","user":{"screen_name":"protogenElvis"}}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.notes.as_deref(), Some("@protogenElvis"));
    }

    #[test]
    fn parse_syndication_response_notes_is_none_when_user_field_absent() {
        let body =
            r#"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null}"#;
        let result = parse_syndication_response(body).unwrap();
        assert_eq!(result.url, "https://pbs.twimg.com/media/abc.jpg:large");
        assert_eq!(result.notes, None);
    }

    #[tokio::test]
    async fn resolve_twitter_status_from_api_url_returns_photo_url_on_success() {
        async fn ok_photo() -> Response {
            (
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"photos":[{"url":"https://pbs.twimg.com/media/abc.jpg:large"}],"video":null}"#,
            )
                .into_response()
        }
        let app = Router::new().route("/tweet-result", get(ok_photo));
        let (listener, address) = bind_mock_server().await;
        serve_mock_server(listener, app);

        let result = resolve_twitter_status_from_api_url(&format!("http://{address}/tweet-result"))
            .await
            .unwrap();

        assert_eq!(result.url, "https://pbs.twimg.com/media/abc.jpg:large");
        assert_eq!(result.notes, None);
    }

    #[tokio::test]
    async fn resolve_twitter_status_from_api_url_returns_fetch_failed_on_404() {
        async fn not_found() -> axum::http::StatusCode {
            axum::http::StatusCode::NOT_FOUND
        }
        let app = Router::new().route("/tweet-result", get(not_found));
        let (listener, address) = bind_mock_server().await;
        serve_mock_server(listener, app);

        let result =
            resolve_twitter_status_from_api_url(&format!("http://{address}/tweet-result")).await;

        assert!(matches!(result, Err(ImageUrlValidationError::FetchFailed)));
    }
}
