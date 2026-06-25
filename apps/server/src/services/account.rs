use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedUserData {
    pub buckets: Vec<ExportedBucket>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedBucket {
    pub name: String,
    pub images: Vec<ExportedImage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedImage {
    pub url: String,
    pub title: Option<String>,
    pub favorite: bool,
    #[serde(rename = "randomWeight")]
    pub random_weight: i64,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
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
        let bucket_rows = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM buckets WHERE owner_user_id = ? ORDER BY name COLLATE NOCASE",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut buckets = Vec::with_capacity(bucket_rows.len());
        for (bucket_id, name) in bucket_rows {
            let image_rows = sqlx::query_as::<
                _,
                (
                    String,
                    String,
                    Option<String>,
                    bool,
                    i64,
                    Option<String>,
                    String,
                ),
            >(
                "SELECT id, url, title, favorite, random_weight, notes, created_at
                 FROM images
                 WHERE owner_user_id = ? AND bucket_id = ?
                 ORDER BY created_at",
            )
            .bind(user_id.to_string())
            .bind(&bucket_id)
            .fetch_all(&self.pool)
            .await?;

            let mut images = Vec::with_capacity(image_rows.len());
            for (img_id, url, title, favorite, random_weight, notes, created_at) in image_rows {
                let tag_rows = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM image_tags WHERE owner_user_id = ? AND image_id = ? ORDER BY position",
                )
                .bind(user_id.to_string())
                .bind(&img_id)
                .fetch_all(&self.pool)
                .await?;

                images.push(ExportedImage {
                    url,
                    title,
                    favorite,
                    random_weight,
                    notes,
                    tags: tag_rows,
                    created_at,
                });
            }

            buckets.push(ExportedBucket { name, images });
        }

        Ok(ExportedUserData { buckets })
    }

    pub async fn import_user_data(
        &self,
        user_id: Uuid,
        data: ExportedUserData,
    ) -> Result<(usize, usize), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let mut buckets_created = 0;
        let mut images_created = 0;

        for exported_bucket in data.buckets {
            let bucket_name = exported_bucket.name.trim();
            if bucket_name.is_empty() {
                continue;
            }

            let name_folded = bucket_name.to_lowercase();
            let bucket_id_str = match sqlx::query_scalar::<_, String>(
                "SELECT id FROM buckets WHERE owner_user_id = ? AND name_folded = ?",
            )
            .bind(user_id.to_string())
            .bind(&name_folded)
            .fetch_optional(&mut *tx)
            .await?
            {
                Some(id) => id,
                None => {
                    let new_id = Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO buckets (id, owner_user_id, name, name_folded) VALUES (?, ?, ?, ?)",
                    )
                    .bind(&new_id)
                    .bind(user_id.to_string())
                    .bind(bucket_name)
                    .bind(&name_folded)
                    .execute(&mut *tx)
                    .await?;
                    buckets_created += 1;
                    new_id
                }
            };

            for exported_image in exported_bucket.images {
                let url = exported_image.url.trim();
                if url.is_empty() {
                    continue;
                }

                let image_exists = sqlx::query_scalar::<_, i64>(
                    "SELECT 1 FROM images WHERE owner_user_id = ? AND bucket_id = ? AND url = ?",
                )
                .bind(user_id.to_string())
                .bind(&bucket_id_str)
                .bind(url)
                .fetch_optional(&mut *tx)
                .await?
                .is_some();

                if image_exists {
                    continue;
                }

                let new_image_id = Uuid::new_v4();

                sqlx::query(
                    "INSERT INTO images (id, owner_user_id, bucket_id, url, title, favorite, random_weight, notes, created_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(new_image_id.to_string())
                .bind(user_id.to_string())
                .bind(&bucket_id_str)
                .bind(url)
                .bind(exported_image.title)
                .bind(exported_image.favorite)
                .bind(exported_image.random_weight)
                .bind(exported_image.notes)
                .bind(exported_image.created_at)
                .execute(&mut *tx)
                .await?;

                let mut position = 0;
                for tag in exported_image.tags {
                    let cleaned = tag
                        .trim()
                        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if cleaned.is_empty() {
                        continue;
                    }
                    let folded = cleaned.to_lowercase();

                    sqlx::query(
                        "INSERT OR IGNORE INTO image_tags (id, owner_user_id, image_id, position, name, name_folded)
                         VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(user_id.to_string())
                    .bind(new_image_id.to_string())
                    .bind(position)
                    .bind(&cleaned)
                    .bind(&folded)
                    .execute(&mut *tx)
                    .await?;

                    position += 1;
                }

                images_created += 1;
            }
        }

        tx.commit().await?;
        Ok((buckets_created, images_created))
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
