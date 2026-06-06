use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct ImageRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredImage {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub pool_id: Uuid,
    pub url: String,
    pub created_at: String,
    pub notes: Option<String>,
}

impl ImageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        url: &str,
    ) -> Result<StoredImage, sqlx::Error> {
        let id = Uuid::new_v4();
        let (stored_id, stored_owner_user_id, stored_pool_id, stored_url, created_at, stored_notes) =
            sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
                r#"
                INSERT INTO images (id, owner_user_id, pool_id, url, notes)
                SELECT ?, ?, id, ?, NULL
                FROM pools
                WHERE id = ? AND owner_user_id = ?
                RETURNING id, owner_user_id, pool_id, url, created_at, notes
                "#,
            )
            .bind(id.to_string())
            .bind(owner_user_id.to_string())
            .bind(url)
            .bind(pool_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_one(&self.pool)
            .await?;

        Ok(StoredImage {
            id: Uuid::parse_str(&stored_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            owner_user_id: Uuid::parse_str(&stored_owner_user_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            pool_id: Uuid::parse_str(&stored_pool_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            url: stored_url,
            created_at,
            notes: stored_notes,
        })
    }

    pub async fn list_for_pool(
        &self,
        user_id: Uuid,
        pool_id: Uuid,
    ) -> Result<Vec<StoredImage>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
            "SELECT id, owner_user_id, pool_id, url, created_at, notes 
             FROM images 
             WHERE pool_id = ? 
               AND (owner_user_id = ? OR EXISTS (SELECT 1 FROM pool_subscriptions WHERE pool_id = images.pool_id AND subscriber_user_id = ?)) 
             ORDER BY created_at",
        )
        .bind(pool_id.to_string())
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, owner, pool, url, created_at, notes)| {
                Ok(StoredImage {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    pool_id: Uuid::parse_str(&pool)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    url,
                    created_at,
                    notes,
                })
            })
            .collect()
    }

    pub async fn delete_for_user(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM images WHERE owner_user_id = ? AND pool_id = ? AND id = ?")
                .bind(owner_user_id.to_string())
                .bind(pool_id.to_string())
                .bind(image_id.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn update_notes(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
        notes: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE images SET notes = ? WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
        )
        .bind(notes)
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }
}
