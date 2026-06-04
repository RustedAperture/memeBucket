use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ExportedUserData {
    pub categories: Vec<ExportedCategory>,
}

#[derive(Debug, Serialize)]
pub struct ExportedCategory {
    pub name: String,
    pub links: Vec<ExportedMediaLink>,
}

#[derive(Debug, Serialize)]
pub struct ExportedMediaLink {
    pub url: String,
}

#[derive(Clone)]
pub struct AccountService {
    pool: SqlitePool,
}

impl AccountService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn export_user_data(&self, user_id: Uuid) -> Result<ExportedUserData, sqlx::Error> {
        let category_rows = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM categories WHERE owner_user_id = ? ORDER BY name COLLATE NOCASE",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut categories = Vec::with_capacity(category_rows.len());
        for (category_id, name) in category_rows {
            let link_rows = sqlx::query_as::<_, (String,)>(
                "SELECT url FROM media_links WHERE owner_user_id = ? AND category_id = ? ORDER BY created_at",
            )
            .bind(user_id.to_string())
            .bind(category_id)
            .fetch_all(&self.pool)
            .await?;

            categories.push(ExportedCategory {
                name,
                links: link_rows
                    .into_iter()
                    .map(|(url,)| ExportedMediaLink { url })
                    .collect(),
            });
        }

        Ok(ExportedUserData { categories })
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
