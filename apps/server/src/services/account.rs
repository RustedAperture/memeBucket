use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ExportedUserData {
    pub pools: Vec<ExportedPool>,
}

#[derive(Debug, Serialize)]
pub struct ExportedPool {
    pub name: String,
    pub images: Vec<ExportedImage>,
}

#[derive(Debug, Serialize)]
pub struct ExportedImage {
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
        let pool_rows = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM pools WHERE owner_user_id = ? ORDER BY name COLLATE NOCASE",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut pools = Vec::with_capacity(pool_rows.len());
        for (pool_id, name) in pool_rows {
            let image_rows = sqlx::query_as::<_, (String,)>(
                "SELECT url FROM images WHERE owner_user_id = ? AND pool_id = ? ORDER BY created_at",
            )
            .bind(user_id.to_string())
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;

            pools.push(ExportedPool {
                name,
                images: image_rows
                    .into_iter()
                    .map(|(url,)| ExportedImage { url })
                    .collect(),
            });
        }

        Ok(ExportedUserData { pools })
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
