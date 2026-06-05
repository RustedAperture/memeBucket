use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub static_dir: PathBuf,
    discord_public_key: String,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            static_dir: PathBuf::from("apps/web/out"),
            discord_public_key: String::new(),
        }
    }

    pub fn for_tests(pool: SqlitePool) -> Self {
        Self::new(pool)
    }

    pub fn with_static_dir(mut self, dir: PathBuf) -> Self {
        self.static_dir = dir;
        self
    }

    pub fn with_discord_public_key(mut self, discord_public_key: String) -> Self {
        self.discord_public_key = discord_public_key;
        self
    }

    pub fn discord_public_key(&self) -> &str {
        &self.discord_public_key
    }
}
