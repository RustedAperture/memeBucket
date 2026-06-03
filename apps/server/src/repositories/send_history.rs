use sqlx::SqlitePool;
use uuid::Uuid;

use crate::repositories::{categories::StoredCategory, media_links::StoredMediaLink};

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
        category: &StoredCategory,
        media_link: &StoredMediaLink,
        visibility: &str,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO send_history
                (id, owner_user_id, category_id, media_link_id, category_name, url, response_visibility)
            SELECT ?, categories.owner_user_id, categories.id, media_links.id, categories.name, media_links.url, ?
            FROM categories
            INNER JOIN media_links
                ON media_links.id = ?
               AND media_links.category_id = categories.id
               AND media_links.owner_user_id = categories.owner_user_id
            WHERE categories.id = ? AND categories.owner_user_id = ?
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(visibility)
        .bind(media_link.id.to_string())
        .bind(category.id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }
}
