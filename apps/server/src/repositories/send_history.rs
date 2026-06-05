use sqlx::SqlitePool;
use uuid::Uuid;

use crate::repositories::{pools::StoredPool, images::StoredImage};

#[derive(Clone)]
pub struct SendHistoryRepository {
    pool: SqlitePool,
}

impl SendHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        owner_user_id: Uuid,
        pool: &StoredPool,
        image: &StoredImage,
        visibility: &str,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO send_history
                (id, owner_user_id, pool_id, image_id, pool_name, url, response_visibility)
            SELECT ?, pools.owner_user_id, pools.id, images.id, pools.name, images.url, ?
            FROM pools
            INNER JOIN images
                ON images.id = ?
               AND images.pool_id = pools.id
               AND images.owner_user_id = pools.owner_user_id
            WHERE pools.id = ? AND pools.owner_user_id = ?
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(visibility)
        .bind(image.id.to_string())
        .bind(pool.id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }
}
