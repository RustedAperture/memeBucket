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
}

impl PoolRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<StoredPool, sqlx::Error> {
        let id = Uuid::new_v4();
        let trimmed_name = name.trim();
        let name_folded = trimmed_name.to_lowercase();

        let (stored_id, stored_owner_user_id, stored_name) =
            sqlx::query_as::<_, (String, String, String)>(
                "INSERT INTO pools (id, owner_user_id, name, name_folded)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(owner_user_id, name_folded) DO NOTHING
                 RETURNING id, owner_user_id, name",
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
        })
    }

    pub async fn list_for_user(
        &self,
        owner_user_id: Uuid,
    ) -> Result<Vec<StoredPool>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, owner_user_id, name FROM pools WHERE owner_user_id = ? ORDER BY name COLLATE NOCASE",
        )
        .bind(owner_user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, owner, name)| {
                Ok(StoredPool {
                    id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    owner_user_id: Uuid::parse_str(&owner)
                        .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                    name,
                })
            })
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
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, owner_user_id, name FROM pools WHERE owner_user_id = ? AND name_folded = ?",
        )
        .bind(owner_user_id.to_string())
        .bind(name_folded)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|(id, owner, name)| {
            Ok(StoredPool {
                id: Uuid::parse_str(&id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                owner_user_id: Uuid::parse_str(&owner)
                    .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
                name,
            })
        })
        .transpose()
    }
}
