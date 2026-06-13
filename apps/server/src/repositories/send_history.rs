use sqlx::SqlitePool;
use std::collections::HashMap;
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

    pub async fn count_for_images(
        &self,
        requester_user_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, i64>, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = String::from(
            "SELECT image_id, COUNT(*) as count
             FROM send_history
             WHERE owner_user_id = ?
               AND image_id IN (",
        );
        for index in 0..image_ids.len() {
            if index > 0 {
                query.push_str(", ");
            }
            query.push('?');
        }
        query.push_str(") GROUP BY image_id");

        let mut built =
            sqlx::query_as::<_, (String, i64)>(&query).bind(requester_user_id.to_string());
        for image_id in image_ids {
            built = built.bind(image_id.to_string());
        }

        let rows = built.fetch_all(&self.pool).await?;
        let mut counts = HashMap::new();
        for (image_id, count) in rows {
            counts.insert(
                Uuid::parse_str(&image_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                count,
            );
        }

        Ok(counts)
    }
}
