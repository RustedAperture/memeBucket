use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::repositories::{buckets::StoredBucket, images::StoredImage};

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
        bucket: &StoredBucket,
        image: &StoredImage,
        visibility: &str,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO send_history
                (id, owner_user_id, bucket_id, image_id, bucket_name, url, response_visibility)
            SELECT ?, ?, buckets.id, images.id, buckets.name, images.url, ?
            FROM buckets
            INNER JOIN images
                ON images.id = ?
               AND images.bucket_id = buckets.id
               AND images.owner_user_id = buckets.owner_user_id
            WHERE buckets.id = ?
              AND (
                buckets.owner_user_id = ?
                OR EXISTS (
                  SELECT 1
                  FROM bucket_subscriptions ps
                  WHERE ps.bucket_id = buckets.id
                    AND ps.subscriber_user_id = ?
                    AND (
                      buckets.whitelist_enabled = 0
                      OR EXISTS (
                        SELECT 1
                        FROM bucket_whitelists w
                        WHERE w.bucket_id = buckets.id AND w.user_id = ?
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
        .bind(bucket.id.to_string())
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

    pub async fn recent_image_ids_for_buckets(
        &self,
        requester_user_id: Uuid,
        bucket_ids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        if bucket_ids.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let placeholders = vec!["?"; bucket_ids.len()].join(", ");
        let sql = format!(
            "SELECT image_id FROM send_history
             WHERE owner_user_id = ?
               AND image_id IS NOT NULL
               AND bucket_id IN ({placeholders})
             ORDER BY sent_at DESC, rowid DESC
             LIMIT ?"
        );

        let mut query = sqlx::query_scalar::<_, String>(&sql).bind(requester_user_id.to_string());
        for bucket_id in bucket_ids {
            query = query.bind(bucket_id.to_string());
        }
        query = query.bind(limit as i64);

        let rows = query.fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|id| Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err))))
            .collect()
    }
}
