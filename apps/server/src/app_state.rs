use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn for_tests(pool: SqlitePool) -> Self {
        Self::new(pool)
    }
}
