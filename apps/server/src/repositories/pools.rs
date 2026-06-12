use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PoolRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredPool {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub share_token: Option<String>,
    pub subscriber_count: i64,
    pub owner_username: Option<String>,
    pub whitelist_enabled: bool,
}

impl PoolRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, owner_user_id: Uuid, name: &str) -> Result<StoredPool, sqlx::Error> {
        let id = Uuid::new_v4();
        let trimmed_name = name.trim();
        let name_folded = trimmed_name.to_lowercase();

        let (stored_id, stored_owner_user_id, stored_name, stored_share_token) =
            sqlx::query_as::<_, (String, String, String, Option<String>)>(
                "INSERT INTO pools (id, owner_user_id, name, name_folded)
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

        Ok(StoredPool {
            id: Uuid::parse_str(&stored_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            owner_user_id: Uuid::parse_str(&stored_owner_user_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            name: stored_name,
            share_token: stored_share_token,
            subscriber_count: 0,
            owner_username: None, // Not needed for newly created owned pool
            whitelist_enabled: false,
        })
    }

    pub async fn rename_pool(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
        new_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let trimmed_name = new_name.trim();
        let name_folded = trimmed_name.to_lowercase();

        let result = sqlx::query(
            "UPDATE pools SET name = ?, name_folded = ? WHERE id = ? AND owner_user_id = ?",
        )
        .bind(trimmed_name)
        .bind(&name_folded)
        .bind(pool_id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_for_user(&self, owner_user_id: Uuid) -> Result<Vec<StoredPool>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token, 
               (SELECT COUNT(*) FROM pool_subscriptions s WHERE s.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p 
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
                )| {
                    Ok(StoredPool {
                        id: Uuid::parse_str(&id)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        owner_user_id: Uuid::parse_str(&owner)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        name,
                        share_token,
                        subscriber_count,
                        owner_username,
                        whitelist_enabled,
                    })
                },
            )
            .collect()
    }

    pub async fn delete_for_user(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM pools WHERE owner_user_id = ? AND id = ?")
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn find_by_name_folded(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredPool>, sqlx::Error> {
        let name_folded = name.trim().to_lowercase();
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM pool_subscriptions s WHERE s.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p 
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
            )| {
                Ok(StoredPool {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                })
            },
        )
        .transpose()
    }

    pub async fn find_accessible_by_name_folded(
        &self,
        user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredPool>, sqlx::Error> {
        let name_folded = name.trim().to_lowercase();
        // First try to find owned pool
        if let Some(pool) = self.find_by_name_folded(user_id, name).await? {
            return Ok(Some(pool));
        }

        // Then try subscribed pools
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM pool_subscriptions s2 WHERE s2.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p
             INNER JOIN pool_subscriptions ps ON p.id = ps.pool_id
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE ps.subscriber_user_id = ?
               AND p.name_folded = ?
               AND (
                 p.whitelist_enabled = 0
                 OR EXISTS (
                   SELECT 1
                   FROM pool_whitelists w
                   WHERE w.pool_id = p.id AND w.user_id = ?
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
            )| {
                Ok(StoredPool {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                })
            },
        )
        .transpose()
    }

    pub async fn get_by_id(&self, pool_id: Uuid) -> Result<Option<StoredPool>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM pool_subscriptions s WHERE s.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p 
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE p.id = ?",
        )
        .bind(pool_id.to_string())
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
            )| {
                Ok(StoredPool {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                })
            },
        )
        .transpose()
    }

    pub async fn get_by_share_token(&self, token: &str) -> Result<Option<StoredPool>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM pool_subscriptions s WHERE s.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p 
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
            )| {
                Ok(StoredPool {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                    share_token,
                    subscriber_count,
                    owner_username,
                    whitelist_enabled,
                })
            },
        )
        .transpose()
    }

    pub async fn set_share_token(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
        token: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE pools SET share_token = ? WHERE id = ? AND owner_user_id = ?")
                .bind(token)
                .bind(pool_id.to_string())
                .bind(owner_user_id.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn subscribe_user_to_pool(
        &self,
        subscriber_user_id: Uuid,
        pool_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO pool_subscriptions (subscriber_user_id, pool_id) VALUES (?, ?) ON CONFLICT DO NOTHING"
        )
        .bind(subscriber_user_id.to_string())
        .bind(pool_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn unsubscribe_user_from_pool(
        &self,
        subscriber_user_id: Uuid,
        pool_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM pool_subscriptions WHERE subscriber_user_id = ? AND pool_id = ?",
        )
        .bind(subscriber_user_id.to_string())
        .bind(pool_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_subscribed_for_user(
        &self,
        subscriber_user_id: Uuid,
    ) -> Result<Vec<StoredPool>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, i64, Option<String>, bool)>(
            "SELECT p.id, p.owner_user_id, p.name, p.share_token,
               (SELECT COUNT(*) FROM pool_subscriptions s2 WHERE s2.pool_id = p.id) as subscriber_count,
               u.username as owner_username,
               p.whitelist_enabled
             FROM pools p
             JOIN pool_subscriptions s ON s.pool_id = p.id
             LEFT JOIN users u ON p.owner_user_id = u.id
             WHERE s.subscriber_user_id = ?
               AND (
                 p.whitelist_enabled = 0
                 OR EXISTS (
                   SELECT 1
                   FROM pool_whitelists w
                   WHERE w.pool_id = p.id AND w.user_id = ?
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
                )| {
                    Ok(StoredPool {
                        id: Uuid::parse_str(&id)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        owner_user_id: Uuid::parse_str(&owner)
                            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                        name,
                        share_token,
                        subscriber_count,
                        owner_username,
                        whitelist_enabled,
                    })
                },
            )
            .collect()
    }

    pub async fn set_whitelist_enabled(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
        enabled: bool,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE pools SET whitelist_enabled = ? WHERE id = ? AND owner_user_id = ?",
        )
        .bind(enabled)
        .bind(pool_id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn add_whitelist_user(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        // First verify the pool belongs to the owner
        let pool_exists = sqlx::query("SELECT 1 FROM pools WHERE id = ? AND owner_user_id = ?")
            .bind(pool_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if pool_exists.is_none() {
            return Ok(false);
        }

        // Lookup user by username (case-insensitive)
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let Some((target_user_id,)) = row else {
            return Ok(false); // User not found
        };

        // Insert into whitelist
        sqlx::query("INSERT OR IGNORE INTO pool_whitelists (pool_id, user_id) VALUES (?, ?)")
            .bind(pool_id.to_string())
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }

    pub async fn remove_whitelist_user(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        // First verify the pool belongs to the owner
        let pool_exists = sqlx::query("SELECT 1 FROM pools WHERE id = ? AND owner_user_id = ?")
            .bind(pool_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if pool_exists.is_none() {
            return Ok(false);
        }

        // Lookup user by username
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let Some((target_user_id,)) = row else {
            return Ok(false);
        };

        let result = sqlx::query("DELETE FROM pool_whitelists WHERE pool_id = ? AND user_id = ?")
            .bind(pool_id.to_string())
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn list_whitelist_users(
        &self,
        pool_id: Uuid,
        owner_user_id: Uuid,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        let pool_exists = sqlx::query("SELECT 1 FROM pools WHERE id = ? AND owner_user_id = ?")
            .bind(pool_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if pool_exists.is_none() {
            return Ok(None);
        }

        let rows = sqlx::query_as::<_, (String,)>("SELECT u.username FROM pool_whitelists w JOIN users u ON w.user_id = u.id WHERE w.pool_id = ? ORDER BY u.username COLLATE NOCASE")
            .bind(pool_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        Ok(Some(rows.into_iter().map(|r| r.0).collect()))
    }

    pub async fn is_user_whitelisted(
        &self,
        pool_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let row = sqlx::query("SELECT 1 FROM pool_whitelists WHERE pool_id = ? AND user_id = ?")
            .bind(pool_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.is_some())
    }
}
