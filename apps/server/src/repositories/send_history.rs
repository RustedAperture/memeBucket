use sqlx::SqlitePool;
use uuid::Uuid;

use crate::repositories::{images::StoredImage, pools::StoredPool};

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
        requester_user_id: Uuid,
        pool: &StoredPool,
        image: &StoredImage,
        visibility: &str,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO send_history
                (id, owner_user_id, pool_id, image_id, pool_name, url, response_visibility)
            SELECT ?, ?, pools.id, images.id, pools.name, images.url, ?
            FROM pools
            INNER JOIN images
                ON images.id = ?
               AND images.pool_id = pools.id
               AND images.owner_user_id = pools.owner_user_id
            WHERE pools.id = ?
              AND (
                pools.owner_user_id = ?
                OR EXISTS (
                  SELECT 1
                  FROM pool_subscriptions ps
                  WHERE ps.pool_id = pools.id
                    AND ps.subscriber_user_id = ?
                    AND (
                      pools.whitelist_enabled = 0
                      OR EXISTS (
                        SELECT 1
                        FROM pool_whitelists w
                        WHERE w.pool_id = pools.id AND w.user_id = ?
                      )
                    )
                )
              )
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(requester_user_id.to_string())
        .bind(visibility)
        .bind(image.id.to_string())
        .bind(pool.id.to_string())
        .bind(requester_user_id.to_string())
        .bind(requester_user_id.to_string())
        .bind(requester_user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }
}
