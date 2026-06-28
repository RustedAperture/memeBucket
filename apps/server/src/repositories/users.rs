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
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub username: Option<String>,
    pub role: String,
}

#[derive(Clone, Debug)]
pub struct StoredIdentity {
    pub id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[async_trait::async_trait]
pub trait UserRepo: Send + Sync {
    /// Look up user by provider identity; create user + identity row if new.
    async fn upsert_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
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

    async fn get_identities(&self, user_id: Uuid) -> Result<Vec<StoredIdentity>, sqlx::Error>;

    async fn count_identities(&self, user_id: Uuid) -> Result<i64, sqlx::Error>;

    async fn link_identity(
        &self,
        user_id: Uuid,
        provider: &str,
        provider_user_id: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<(), sqlx::Error>;

    async fn unlink_identity(&self, user_id: Uuid, provider: &str) -> Result<(), sqlx::Error>;
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepo for UserRepository {
    async fn upsert_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<StoredUser, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Try to find existing identity
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM user_identities WHERE provider = ? AND provider_user_id = ?",
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let user_id = if let Some((uid,)) = existing {
            // Update display_name/avatar on the identity row
            sqlx::query(
                "UPDATE user_identities SET display_name = ?, avatar_url = ? WHERE provider = ? AND provider_user_id = ?"
            )
            .bind(display_name)
            .bind(avatar_url)
            .bind(provider)
            .bind(provider_user_id)
            .execute(&mut *tx)
            .await?;
            uid
        } else {
            // Create new user
            let new_user_id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO users (id, display_name, avatar_url) VALUES (?, ?, ?)")
                .bind(&new_user_id)
                .bind(display_name)
                .bind(avatar_url)
                .execute(&mut *tx)
                .await?;

            // Create identity row
            let identity_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO user_identities (id, user_id, provider, provider_user_id, display_name, avatar_url) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&identity_id)
            .bind(&new_user_id)
            .bind(provider)
            .bind(provider_user_id)
            .bind(display_name)
            .bind(avatar_url)
            .execute(&mut *tx)
            .await?;

            new_user_id
        };

        let (id, display_name, avatar_url, username, role): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ) = sqlx::query_as(
            "SELECT id, display_name, avatar_url, username, role FROM users WHERE id = ?",
        )
        .bind(&user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(StoredUser {
            id: Uuid::parse_str(&id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            display_name,
            avatar_url,
            username,
            role,
        })
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<StoredUser>, sqlx::Error> {
        let row: Option<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        )> = sqlx::query_as(
            "SELECT id, display_name, avatar_url, username, role FROM users WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|(id, display_name, avatar_url, username, role)| {
            Ok(StoredUser {
                id: Uuid::parse_str(&id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
                display_name,
                avatar_url,
                username,
                role,
            })
        })
        .transpose()
    }

    async fn update_username(&self, id: Uuid, username: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users SET username = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
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

    async fn get_identities(&self, user_id: Uuid) -> Result<Vec<StoredIdentity>, sqlx::Error> {
        let rows: Vec<(String, String, String, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, provider, provider_user_id, display_name, avatar_url FROM user_identities WHERE user_id = ? ORDER BY linked_at"
            )
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(
                |(id, provider, provider_user_id, display_name, avatar_url)| {
                    Ok(StoredIdentity {
                        id: Uuid::parse_str(&id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
                        provider,
                        provider_user_id,
                        display_name,
                        avatar_url,
                    })
                },
            )
            .collect()
    }

    async fn count_identities(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM user_identities WHERE user_id = ?")
                .bind(user_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    async fn link_identity(
        &self,
        user_id: Uuid,
        provider: &str,
        provider_user_id: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO user_identities (id, user_id, provider, provider_user_id, display_name, avatar_url) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(provider)
        .bind(provider_user_id)
        .bind(display_name)
        .bind(avatar_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn unlink_identity(&self, user_id: Uuid, provider: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_identities WHERE user_id = ? AND provider = ?")
            .bind(user_id.to_string())
            .bind(provider)
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
                    .bind(&new_id).bind(user_id.to_string()).bind(bucket_name).bind(&name_folded)
                    .execute(&mut *tx).await?;
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
                .bind(new_image_id.to_string()).bind(user_id.to_string()).bind(&bucket_id_str)
                .bind(url).bind(exported_image.title).bind(exported_image.favorite)
                .bind(exported_image.random_weight).bind(exported_image.notes)
                .bind(exported_image.created_at)
                .execute(&mut *tx).await?;

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
                    .bind(Uuid::new_v4().to_string()).bind(user_id.to_string())
                    .bind(new_image_id.to_string()).bind(position).bind(&cleaned).bind(&folded)
                    .execute(&mut *tx).await?;
                    position += 1;
                }
                images_created += 1;
            }
        }

        tx.commit().await?;
        Ok((buckets_created, images_created))
    }
}
