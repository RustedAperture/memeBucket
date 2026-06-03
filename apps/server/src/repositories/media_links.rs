use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct MediaLinkRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredMediaLink {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub category_id: Uuid,
    pub url: String,
}

impl MediaLinkRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_user_id: Uuid,
        category_id: Uuid,
        url: &str,
    ) -> Result<StoredMediaLink, sqlx::Error> {
        let id = Uuid::new_v4();
        let (stored_id, stored_owner_user_id, stored_category_id, stored_url) =
            sqlx::query_as::<_, (String, String, String, String)>(
                r#"
                INSERT INTO media_links (id, owner_user_id, category_id, url)
                SELECT ?, ?, id, ?
                FROM categories
                WHERE id = ? AND owner_user_id = ?
                RETURNING id, owner_user_id, category_id, url
                "#,
            )
            .bind(id.to_string())
            .bind(owner_user_id.to_string())
            .bind(url)
            .bind(category_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_one(&self.pool)
            .await?;

        Ok(StoredMediaLink {
            id: Uuid::parse_str(&stored_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            owner_user_id: Uuid::parse_str(&stored_owner_user_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            category_id: Uuid::parse_str(&stored_category_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            url: stored_url,
        })
    }
}
