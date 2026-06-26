use sqlx::SqlitePool;
use uuid::Uuid;

use crate::services::account::ExportedUserData;

#[derive(Clone)]
pub struct UserRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredUser {
    pub id: Uuid,
    pub discord_user_key: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub username: Option<String>,
}

#[async_trait::async_trait]
pub trait UserRepo: Send + Sync {
    async fn upsert_by_discord_key(
        &self,
        discord_user_key: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<StoredUser, sqlx::Error>;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<StoredUser>, sqlx::Error>;

    async fn update_username(&self, id: Uuid, username: &str) -> Result<bool, sqlx::Error>;

    async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error>;

    async fn import_user_data(
        &self,
        user_id: Uuid,
        data: ExportedUserData,
    ) -> Result<(usize, usize), sqlx::Error>;
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepo for UserRepository {
    async fn upsert_by_discord_key(
        &self,
        discord_user_key: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<StoredUser, sqlx::Error> {
        let (id, discord_user_key, display_name, avatar_url, username) = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            INSERT INTO users (id, discord_user_key, display_name, avatar_url, updated_at)
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(discord_user_key) DO UPDATE SET
            		display_name = excluded.display_name,
            		avatar_url = excluded.avatar_url,
            		updated_at = CURRENT_TIMESTAMP
            RETURNING id, discord_user_key, display_name, avatar_url, username
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(discord_user_key)
        .bind(display_name)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(StoredUser {
            id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            discord_user_key,
            display_name,
            avatar_url,
            username,
        })
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<StoredUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>)>(
            "SELECT id, discord_user_key, display_name, avatar_url, username FROM users WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(
            |(id, discord_user_key, display_name, avatar_url, username)| {
                Ok(StoredUser {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    discord_user_key,
                    display_name,
                    avatar_url,
                    username,
                })
            },
        )
        .transpose()
    }

    async fn update_username(&self, id: Uuid, username: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE users SET username = ? WHERE id = ?")
            .bind(username)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn import_user_data(
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
}
