use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct BucketRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredBucket {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub share_token: Option<String>,
    pub subscriber_count: i64,
    pub owner_username: Option<String>,
    pub whitelist_enabled: bool,
    pub image_count: i64,
}

impl BucketRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<StoredBucket, sqlx::Error> {
        let id = Uuid::new_v4();
        let trimmed_name = name.trim();
        let name_folded = trimmed_name.to_lowercase();

        let (stored_id, stored_owner_user_id, stored_name, stored_share_token) =
            sqlx::query_as::<_, (String, String, String, Option<String>)>(
                "INSERT INTO buckets (id, owner_user_id, name, name_folded)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(owner_user_id, name_folded) DO NOTHING
                 RETURNING id, owner_user_id, name, share_token",
            )
            .bind(id.to_string())
            .bind(owner_user_id.to_string())
            .bind(trimmed_name)
            .bind(&name_folded)
            .fetch_one(&self.pool)
            .await?;

        Ok(StoredBucket {
            id: Uuid::parse_str(&stored_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            owner_user_id: Uuid::parse_str(&stored_owner_user_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            name: stored_name,
            share_token: stored_share_token,
            subscriber_count: 0,
            owner_username: None,
            whitelist_enabled: false,
            image_count: 0,
        })
    }

    pub async fn rename_bucket(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        new_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let trimmed_name = new_name.trim();
        let name_folded = trimmed_name.to_lowercase();

        let result = sqlx::query(
            "UPDATE buckets SET name = ?, name_folded = ? WHERE id = ? AND owner_user_id = ?",
        )
        .bind(trimmed_name)
        .bind(&name_folded)
        .bind(bucket_id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_for_user(
        &self,
        owner_user_id: Uuid,
    ) -> Result<Vec<StoredBucket>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token, 
               (SELECT COUNT(*) FROM bucket_subscriptions s WHERE s.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p 
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE p.owner_user_id = ? ORDER BY p.name COLLATE NOCASE",
        )
        .bind(owner_user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(
                |(
                    id,
                    owner,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                )| {
                    Ok(StoredBucket {
                        id: Uuid::parse_str(&id)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        owner_user_id: Uuid::parse_str(&owner)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        name,
                        share_token,
                        subscriber_count,
                        owner_username,
                        whitelist_enabled,
                        image_count,
                    })
                },
            )
            .collect()
    }

    pub async fn list_bucket_names_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM (
                  SELECT name FROM buckets WHERE owner_user_id = ?
                  UNION
                  SELECT p.name FROM buckets p JOIN bucket_subscriptions s ON s.bucket_id = p.id WHERE s.subscriber_user_id = ?
              ) ORDER BY name COLLATE NOCASE",
        )
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    pub async fn delete_for_user(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM buckets WHERE owner_user_id = ? AND id = ?")
            .bind(owner_user_id.to_string())
            .bind(bucket_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn find_by_name_folded(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        let name_folded = name.trim().to_lowercase();
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM bucket_subscriptions s WHERE s.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p 
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE p.owner_user_id = ? AND p.name_folded = ?",
        )
        .bind(owner_user_id.to_string())
        .bind(name_folded)
        .fetch_optional(&self.pool)
        .await?;

        row.map(
            |(
                id,
                owner,
                name,
                share_token,
                subscriber_count,
                owner_username,
                whitelist_enabled,
                image_count,
            )| {
                Ok(StoredBucket {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                })
            },
        )
        .transpose()
    }

    pub async fn find_accessible_by_name_folded(
        &self,
        user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        let name_folded = name.trim().to_lowercase();
        if let Some(bucket) = self.find_by_name_folded(user_id, name).await? {
            return Ok(Some(bucket));
        }

        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM bucket_subscriptions s2 WHERE s2.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p
             INNER JOIN bucket_subscriptions ps ON p.id = ps.bucket_id
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE ps.subscriber_user_id = ?
               AND p.name_folded = ?
               AND (
                 p.whitelist_enabled = 0
                 OR EXISTS (
                   SELECT 1
                   FROM bucket_whitelists w
                   WHERE w.bucket_id = p.id AND w.user_id = ?
                 )
               )",
        )
        .bind(user_id.to_string())
        .bind(name_folded)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(
            |(
                id,
                owner,
                name,
                share_token,
                subscriber_count,
                owner_username,
                whitelist_enabled,
                image_count,
            )| {
                Ok(StoredBucket {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                })
            },
        )
        .transpose()
    }

    pub async fn get_by_id(&self, bucket_id: Uuid) -> Result<Option<StoredBucket>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM bucket_subscriptions s WHERE s.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p 
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE p.id = ?",
        )
        .bind(bucket_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(
            |(
                id,
                owner,
                name,
                share_token,
                subscriber_count,
                owner_username,
                whitelist_enabled,
                image_count,
            )| {
                Ok(StoredBucket {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                })
            },
        )
        .transpose()
    }

    pub async fn get_by_share_token(
        &self,
        token: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM bucket_subscriptions s WHERE s.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p 
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE p.share_token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        row.map(
            |(
                id,
                owner,
                name,
                share_token,
                subscriber_count,
                owner_username,
                whitelist_enabled,
                image_count,
            )| {
                Ok(StoredBucket {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                })
            },
        )
        .transpose()
    }

    pub async fn set_share_token(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        token: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE buckets SET share_token = ? WHERE id = ? AND owner_user_id = ?")
                .bind(token)
                .bind(bucket_id.to_string())
                .bind(owner_user_id.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn subscribe_user_to_bucket(
        &self,
        subscriber_user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO bucket_subscriptions (subscriber_user_id, bucket_id) VALUES (?, ?) ON CONFLICT DO NOTHING"
        )
        .bind(subscriber_user_id.to_string())
        .bind(bucket_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn unsubscribe_user_from_bucket(
        &self,
        subscriber_user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM bucket_subscriptions WHERE subscriber_user_id = ? AND bucket_id = ?",
        )
        .bind(subscriber_user_id.to_string())
        .bind(bucket_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_subscribed_for_user(
        &self,
        subscriber_user_id: Uuid,
    ) -> Result<Vec<StoredBucket>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool, i64)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM bucket_subscriptions s2 WHERE s2.bucket_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled,
               (SELECT COUNT(*) FROM images i WHERE i.bucket_id = p.id) as image_count
             FROM buckets p
             JOIN bucket_subscriptions s ON s.bucket_id = p.id
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE s.subscriber_user_id = ?
               AND (
                 p.whitelist_enabled = 0
                 OR EXISTS (
                   SELECT 1
                   FROM bucket_whitelists w
                   WHERE w.bucket_id = p.id AND w.user_id = ?
                 )
               )
             ORDER BY p.name COLLATE NOCASE",
        )
        .bind(subscriber_user_id.to_string())
        .bind(subscriber_user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(
                |(
                    id,
                    owner,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                    image_count,
                )| {
                    Ok(StoredBucket {
                        id: Uuid::parse_str(&id)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        owner_user_id: Uuid::parse_str(&owner)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        name,
                        share_token,
                        subscriber_count,
                        owner_username,
                        whitelist_enabled,
                        image_count,
                    })
                },
            )
            .collect()
    }

    pub async fn set_whitelist_enabled(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        enabled: bool,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE buckets SET whitelist_enabled = ? WHERE id = ? AND owner_user_id = ?",
        )
        .bind(enabled)
        .bind(bucket_id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn add_whitelist_user(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        let bucket_exists = sqlx::query("SELECT 1 FROM buckets WHERE id = ? AND owner_user_id = ?")
            .bind(bucket_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if bucket_exists.is_none() {
            return Ok(false);
        }

        let row = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let Some((target_user_id,)) = row else {
            return Ok(false);
        };

        sqlx::query("INSERT OR IGNORE INTO bucket_whitelists (bucket_id, user_id) VALUES (?, ?)")
            .bind(bucket_id.to_string())
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }

    pub async fn remove_whitelist_user(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        let bucket_exists = sqlx::query("SELECT 1 FROM buckets WHERE id = ? AND owner_user_id = ?")
            .bind(bucket_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if bucket_exists.is_none() {
            return Ok(false);
        }

        let row = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let Some((target_user_id,)) = row else {
            return Ok(false);
        };

        let result =
            sqlx::query("DELETE FROM bucket_whitelists WHERE bucket_id = ? AND user_id = ?")
                .bind(bucket_id.to_string())
                .bind(target_user_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_whitelist_users(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        let bucket_exists = sqlx::query("SELECT 1 FROM buckets WHERE id = ? AND owner_user_id = ?")
            .bind(bucket_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if bucket_exists.is_none() {
            return Ok(None);
        }

        let rows = sqlx::query_as::<_, (String,)>("SELECT u.username FROM bucket_whitelists w JOIN users u ON w.user_id = u.id WHERE w.bucket_id = ? ORDER BY u.username COLLATE NOCASE")
            .bind(bucket_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        Ok(Some(rows.into_iter().map(|r| r.0).collect()))
    }

    pub async fn is_user_whitelisted(
        &self,
        bucket_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let row =
            sqlx::query("SELECT 1 FROM bucket_whitelists WHERE bucket_id = ? AND user_id = ?")
                .bind(bucket_id.to_string())
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.is_some())
    }
}
