use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    discord_public_key: String,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            discord_public_key: String::new(),
        }
    }

    pub fn for_tests(pool: SqlitePool) -> Self {
        Self::new(pool)
    }

    pub fn with_discord_public_key(mut self, discord_public_key: String) -> Self {
        self.discord_public_key = discord_public_key;
        self
    }

    pub fn discord_public_key(&self) -> &str {
        &self.discord_public_key
    }
}
