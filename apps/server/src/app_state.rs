use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct GifSearchCacheEntry {
    pub value: serde_json::Value,
    pub expires_at: Instant,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub static_dir: PathBuf,
    pub session_secret: String,
    discord_public_key: String,
    pub klipy_api_key: Option<String>,
    pub imgbb_api_key: Option<String>,
    pub klipy_api_base_url: String,
    pub gif_search_cache: Arc<Mutex<HashMap<String, GifSearchCacheEntry>>>,
    pub gif_search_cache_ttl: Duration,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            static_dir: PathBuf::from("apps/web/out"),
            session_secret: String::new(),
            discord_public_key: String::new(),
            klipy_api_key: None,
            imgbb_api_key: None,
            klipy_api_base_url: "https://api.klipy.com".to_string(),
            gif_search_cache: Arc::new(Mutex::new(HashMap::new())),
            gif_search_cache_ttl: Duration::from_secs(60 * 60 * 6),
        }
    }

    pub fn for_tests(pool: SqlitePool) -> Self {
        Self::new(pool)
    }

    pub fn with_static_dir(mut self, dir: PathBuf) -> Self {
        self.static_dir = dir;
        self
    }

    pub fn with_session_secret(mut self, secret: String) -> Self {
        self.session_secret = secret;
        self
    }

    pub fn session_secret(&self) -> &str {
        &self.session_secret
    }

    pub fn with_discord_public_key(mut self, discord_public_key: String) -> Self {
        self.discord_public_key = discord_public_key;
        self
    }

    pub fn discord_public_key(&self) -> &str {
        &self.discord_public_key
    }

    pub fn with_klipy_api_key(mut self, klipy_api_key: Option<String>) -> Self {
        self.klipy_api_key = klipy_api_key;
        self
    }

    pub fn with_imgbb_api_key(mut self, imgbb_api_key: Option<String>) -> Self {
        self.imgbb_api_key = imgbb_api_key;
        self
    }

    pub fn with_klipy_api_base_url(mut self, klipy_api_base_url: String) -> Self {
        self.klipy_api_base_url = klipy_api_base_url;
        self
    }

    pub fn with_gif_search_cache_ttl(mut self, gif_search_cache_ttl: Duration) -> Self {
        self.gif_search_cache_ttl = gif_search_cache_ttl;
        self
    }
}
