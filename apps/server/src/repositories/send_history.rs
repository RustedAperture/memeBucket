use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SendHistoryRepository {
    _pool: SqlitePool,
}

impl SendHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { _pool: pool }
    }
}
