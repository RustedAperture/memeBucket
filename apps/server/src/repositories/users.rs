use sqlx::SqlitePool;
use uuid::Uuid;

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

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_by_discord_key(
        &self,
        discord_user_key: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<StoredUser, sqlx::Error> {
        let (id, discord_user_key, display_name, avatar_url, username) =
            sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>)>(
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

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<StoredUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>)>(
            "SELECT id, discord_user_key, display_name, avatar_url, username FROM users WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|(id, discord_user_key, display_name, avatar_url, username)| {
            Ok(StoredUser {
                id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                discord_user_key,
                display_name,
                avatar_url,
                username,
            })
        }).transpose()
    }

    pub async fn update_username(&self, id: Uuid, username: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE users SET username = ? WHERE id = ?")
            .bind(username)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
