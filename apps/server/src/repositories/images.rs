use std::collections::{HashMap, HashSet};

use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct ImageRepository {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredImage {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub pool_id: Uuid,
    pub url: String,
    pub title: Option<String>,
    pub favorite: bool,
    pub random_weight: i64,
    pub tags: Vec<String>,
    pub created_at: String,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct UpdateImageMetadataPatch {
    pub title: Option<Option<String>>,
    pub notes: Option<Option<String>>,
    pub favorite: Option<bool>,
    pub random_weight: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct BulkImageMetadataPatch {
    pub image_ids: Vec<Uuid>,
    pub favorite: Option<bool>,
    pub random_weight: Option<i64>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ImageSearchFilters {
    pub query: Option<String>,
    pub pool_id: Option<Uuid>,
    pub favorite: Option<bool>,
    pub random_enabled: Option<bool>,
    pub tags: Vec<String>,
    pub limit: i64,
}

#[derive(Clone, Debug)]
pub struct StoredImageSearchResult {
    pub pool_name: String,
    pub image: StoredImage,
}

type StoredImageRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    bool,
    i64,
    String,
    Option<String>,
);

type SearchImageRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    bool,
    i64,
    String,
    Option<String>,
);

impl ImageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        url: &str,
    ) -> Result<StoredImage, sqlx::Error> {
        self.create_with_metadata(owner_user_id, pool_id, url, None, false, 1, &[])
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_metadata(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        url: &str,
        title: Option<&str>,
        favorite: bool,
        random_weight: i64,
        tags: &[String],
    ) -> Result<StoredImage, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<String>,
                bool,
                i64,
                String,
                Option<String>,
            ),
        >(
                r#"
                INSERT INTO images (id, owner_user_id, pool_id, url, title, favorite, random_weight, notes)
                SELECT ?, ?, id, ?, ?, ?, ?, NULL
                FROM pools
                WHERE id = ? AND owner_user_id = ?
                RETURNING id, owner_user_id, pool_id, url, title, favorite, random_weight, created_at, notes
                "#,
            )
            .bind(id.to_string())
            .bind(owner_user_id.to_string())
            .bind(url)
            .bind(title)
            .bind(favorite)
            .bind(random_weight)
            .bind(pool_id.to_string())
            .bind(owner_user_id.to_string())
            .fetch_one(&mut *tx)
            .await?;

        let normalized_tags = self.replace_tags(&mut tx, owner_user_id, id, tags).await?;
        tx.commit().await?;

        Self::stored_image_from_row(row, normalized_tags)
    }

    pub async fn list_for_pool(
        &self,
        user_id: Uuid,
        pool_id: Uuid,
    ) -> Result<Vec<StoredImage>, sqlx::Error> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<String>,
                bool,
                i64,
                String,
                Option<String>,
            ),
        >(
            "SELECT id, owner_user_id, pool_id, url, title, favorite, random_weight, created_at, notes
             FROM images
             WHERE pool_id = ?
               AND (
                 owner_user_id = ?
                 OR EXISTS (
                   SELECT 1
                   FROM pool_subscriptions ps
                   JOIN pools p ON p.id = ps.pool_id
                   WHERE ps.pool_id = images.pool_id
                     AND ps.subscriber_user_id = ?
                     AND (
                       p.whitelist_enabled = 0
                       OR EXISTS (
                         SELECT 1
                         FROM pool_whitelists w
                         WHERE w.pool_id = p.id AND w.user_id = ?
                       )
                     )
                 )
               )
             ORDER BY created_at",
        )
        .bind(pool_id.to_string())
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let tags_by_image = self.load_tags_for_images(rows.iter()).await?;

        rows.into_iter()
            .map(|row| {
                let image_id =
                    Uuid::parse_str(&row.0).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
                Self::stored_image_from_row(
                    row,
                    tags_by_image.get(&image_id).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    pub async fn search_for_user(
        &self,
        user_id: Uuid,
        filters: &ImageSearchFilters,
    ) -> Result<Vec<StoredImageSearchResult>, sqlx::Error> {
        let mut builder: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
            "SELECT p.name, images.id, images.owner_user_id, images.pool_id, images.url,
                    images.title, images.favorite, images.random_weight, images.created_at, images.notes
             FROM images
             INNER JOIN pools p
                ON p.id = images.pool_id
               AND p.owner_user_id = images.owner_user_id
             WHERE (
                 images.owner_user_id = ",
        );
        builder.push_bind(user_id.to_string());
        builder.push(
            "
                 OR EXISTS (
                   SELECT 1
                   FROM pool_subscriptions ps
                   WHERE ps.pool_id = images.pool_id
                     AND ps.subscriber_user_id = ",
        );
        builder.push_bind(user_id.to_string());
        builder.push(
            "
                     AND (
                       p.whitelist_enabled = 0
                       OR EXISTS (
                         SELECT 1
                         FROM pool_whitelists w
                         WHERE w.pool_id = p.id AND w.user_id = ",
        );
        builder.push_bind(user_id.to_string());
        builder.push(
            "
                       )
                     )
                 )
             )",
        );

        if let Some(pool_id) = filters.pool_id {
            builder.push(" AND images.pool_id = ");
            builder.push_bind(pool_id.to_string());
        }

        if let Some(favorite) = filters.favorite {
            builder.push(" AND images.favorite = ");
            builder.push_bind(favorite);
        }

        if let Some(random_enabled) = filters.random_enabled {
            if random_enabled {
                builder.push(" AND images.random_weight > 0");
            } else {
                builder.push(" AND images.random_weight = 0");
            }
        }

        if let Some(query) = normalized_search_query(&filters.query) {
            let pattern = like_pattern(&query);
            builder.push(
                " AND (
                    images.url LIKE ",
            );
            builder.push_bind(pattern.clone());
            builder.push(" ESCAPE '\\' OR images.title LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" ESCAPE '\\' OR images.notes LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" ESCAPE '\\' OR p.name LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" ESCAPE '\\'");
            builder.push(
                " OR EXISTS (
                    SELECT 1
                    FROM image_tags it
                    WHERE it.image_id = images.id
                      AND it.name_folded LIKE ",
            );
            builder.push_bind(pattern);
            builder.push(" ESCAPE '\\'))");
        }

        for tag in normalized_filter_tags(&filters.tags) {
            builder.push(
                " AND EXISTS (
                    SELECT 1
                    FROM image_tags it
                    WHERE it.image_id = images.id
                      AND it.name_folded = ",
            );
            builder.push_bind(tag);
            builder.push(")");
        }

        builder.push(" ORDER BY images.favorite DESC, images.created_at DESC LIMIT ");
        builder.push_bind(filters.limit.clamp(1, 100));

        let rows = builder
            .build_query_as::<SearchImageRow>()
            .fetch_all(&self.pool)
            .await?;

        let image_ids = rows
            .iter()
            .map(|row| Uuid::parse_str(&row.1).map_err(|err| sqlx::Error::Decode(Box::new(err))))
            .collect::<Result<Vec<_>, _>>()?;
        let tags_by_image = self.load_tags_for_image_ids(&image_ids).await?;

        rows.into_iter()
            .map(|row| {
                let image_id =
                    Uuid::parse_str(&row.1).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
                let image_row = (
                    row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9,
                );
                Ok(StoredImageSearchResult {
                    pool_name: row.0,
                    image: Self::stored_image_from_row(
                        image_row,
                        tags_by_image.get(&image_id).cloned().unwrap_or_default(),
                    )?,
                })
            })
            .collect()
    }

    pub async fn get_for_owner(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
    ) -> Result<Option<StoredImage>, sqlx::Error> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<String>,
                bool,
                i64,
                String,
                Option<String>,
            ),
        >(
            "SELECT id, owner_user_id, pool_id, url, title, favorite, random_weight, created_at, notes
             FROM images
             WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
        )
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let tags_by_image = self.load_tags_for_image_ids(&[image_id]).await?;
        Ok(Some(Self::stored_image_from_row(
            row,
            tags_by_image.get(&image_id).cloned().unwrap_or_default(),
        )?))
    }

    pub async fn delete_for_user(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM images WHERE owner_user_id = ? AND pool_id = ? AND id = ?")
                .bind(owner_user_id.to_string())
                .bind(pool_id.to_string())
                .bind(image_id.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn update_notes(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
        notes: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE images SET notes = ? WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
        )
        .bind(notes)
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_metadata(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
        title: Option<&str>,
        notes: Option<&str>,
        favorite: bool,
        random_weight: i64,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE images
             SET title = ?, notes = ?, favorite = ?, random_weight = ?
             WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
        )
        .bind(title)
        .bind(notes)
        .bind(favorite)
        .bind(random_weight)
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }

        self.replace_tags(&mut tx, owner_user_id, image_id, tags)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn update_metadata_partial(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
        patch: &UpdateImageMetadataPatch,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM images WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
        )
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        if !exists {
            tx.rollback().await?;
            return Ok(false);
        }

        if let Some(title) = &patch.title {
            sqlx::query(
                "UPDATE images SET title = ? WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
            )
            .bind(title.as_deref())
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .bind(image_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        if let Some(notes) = &patch.notes {
            sqlx::query(
                "UPDATE images SET notes = ? WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
            )
            .bind(notes.as_deref())
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .bind(image_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        if let Some(favorite) = patch.favorite {
            sqlx::query(
                "UPDATE images
                 SET favorite = ?
                 WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
            )
            .bind(favorite)
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .bind(image_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        if let Some(random_weight) = patch.random_weight {
            sqlx::query(
                "UPDATE images
                 SET random_weight = ?
                 WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
            )
            .bind(random_weight)
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .bind(image_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        if let Some(tags) = &patch.tags {
            self.replace_tags(&mut tx, owner_user_id, image_id, tags)
                .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    pub async fn update_metadata_bulk(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        patch: &BulkImageMetadataPatch,
    ) -> Result<usize, sqlx::Error> {
        let mut unique_image_ids = Vec::new();
        let mut seen_image_ids = HashSet::new();
        for image_id in &patch.image_ids {
            if seen_image_ids.insert(*image_id) {
                unique_image_ids.push(*image_id);
            }
        }

        if unique_image_ids.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut valid_image_ids = Vec::new();
        for image_id in &unique_image_ids {
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT 1 FROM images WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
            )
            .bind(owner_user_id.to_string())
            .bind(pool_id.to_string())
            .bind(image_id.to_string())
            .fetch_optional(&mut *tx)
            .await?
            .is_some();

            if exists {
                valid_image_ids.push(*image_id);
            }
        }

        if valid_image_ids.len() != unique_image_ids.len() {
            tx.rollback().await?;
            return Ok(0);
        }

        let add_tags = normalized_tag_names(&patch.add_tags);
        let remove_tags = normalized_tag_folds(&patch.remove_tags);
        let should_update_tags = !add_tags.is_empty() || !remove_tags.is_empty();

        for image_id in &valid_image_ids {
            if let Some(favorite) = patch.favorite {
                sqlx::query(
                    "UPDATE images
                     SET favorite = ?
                     WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
                )
                .bind(favorite)
                .bind(owner_user_id.to_string())
                .bind(pool_id.to_string())
                .bind(image_id.to_string())
                .execute(&mut *tx)
                .await?;
            }

            if let Some(random_weight) = patch.random_weight {
                sqlx::query(
                    "UPDATE images
                     SET random_weight = ?
                     WHERE owner_user_id = ? AND pool_id = ? AND id = ?",
                )
                .bind(random_weight)
                .bind(owner_user_id.to_string())
                .bind(pool_id.to_string())
                .bind(image_id.to_string())
                .execute(&mut *tx)
                .await?;
            }

            if should_update_tags {
                let current_tags = sqlx::query_scalar::<_, String>(
                    "SELECT name FROM image_tags WHERE image_id = ? ORDER BY position",
                )
                .bind(image_id.to_string())
                .fetch_all(&mut *tx)
                .await?;
                let mut next_tags = current_tags
                    .into_iter()
                    .filter(|tag| {
                        let Some((_, folded)) = Self::normalize_tag(tag) else {
                            return false;
                        };
                        !remove_tags.contains(&folded)
                    })
                    .collect::<Vec<_>>();

                let mut seen_tags = next_tags
                    .iter()
                    .filter_map(|tag| Self::normalize_tag(tag).map(|(_, folded)| folded))
                    .collect::<HashSet<_>>();

                for tag in &add_tags {
                    let Some((name, folded)) = Self::normalize_tag(tag) else {
                        continue;
                    };
                    if seen_tags.insert(folded) {
                        next_tags.push(name);
                    }
                }

                self.replace_tags(&mut tx, owner_user_id, *image_id, &next_tags)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(valid_image_ids.len())
    }

    pub async fn move_to_pool(
        &self,
        owner_user_id: Uuid,
        pool_id: Uuid,
        image_id: Uuid,
        new_pool_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE images
             SET pool_id = ?
             WHERE owner_user_id = ?
               AND pool_id = ?
               AND id = ?
               AND EXISTS (
                 SELECT 1
                 FROM pools
                 WHERE id = ? AND owner_user_id = ?
               )",
        )
        .bind(new_pool_id.to_string())
        .bind(owner_user_id.to_string())
        .bind(pool_id.to_string())
        .bind(image_id.to_string())
        .bind(new_pool_id.to_string())
        .bind(owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    async fn replace_tags(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        owner_user_id: Uuid,
        image_id: Uuid,
        tags: &[String],
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query("DELETE FROM image_tags WHERE owner_user_id = ? AND image_id = ?")
            .bind(owner_user_id.to_string())
            .bind(image_id.to_string())
            .execute(&mut **tx)
            .await?;

        let mut normalized = Vec::new();
        let mut seen = HashSet::new();
        for tag in tags {
            let Some((name, name_folded)) = Self::normalize_tag(tag) else {
                continue;
            };
            if !seen.insert(name_folded.clone()) {
                continue;
            }

            let position = normalized.len() as i64;

            sqlx::query(
                "INSERT INTO image_tags (id, owner_user_id, image_id, position, name, name_folded)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(owner_user_id.to_string())
            .bind(image_id.to_string())
            .bind(position)
            .bind(&name)
            .bind(&name_folded)
            .execute(&mut **tx)
            .await?;

            normalized.push(name);
        }

        Ok(normalized)
    }

    async fn load_tags_for_images<'a, I>(
        &self,
        rows: I,
    ) -> Result<HashMap<Uuid, Vec<String>>, sqlx::Error>
    where
        I: IntoIterator<
            Item = &'a (
                String,
                String,
                String,
                String,
                Option<String>,
                bool,
                i64,
                String,
                Option<String>,
            ),
        >,
    {
        let mut image_ids = Vec::new();
        for row in rows {
            image_ids
                .push(Uuid::parse_str(&row.0).map_err(|err| sqlx::Error::Decode(Box::new(err)))?);
        }

        if image_ids.is_empty() {
            return Ok(HashMap::new());
        }

        self.load_tags_for_image_ids(&image_ids).await
    }

    async fn load_tags_for_image_ids(
        &self,
        image_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<String>>, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = String::from("SELECT image_id, name FROM image_tags WHERE image_id IN (");
        for index in 0..image_ids.len() {
            if index > 0 {
                query.push_str(", ");
            }
            query.push('?');
        }
        query.push_str(") ORDER BY image_id, position");

        let mut built = sqlx::query_as::<_, (String, String)>(&query);
        for image_id in image_ids {
            built = built.bind(image_id.to_string());
        }

        let rows = built.fetch_all(&self.pool).await?;
        let mut tags_by_image = HashMap::new();
        for (image_id, name) in rows {
            let image_id =
                Uuid::parse_str(&image_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
            tags_by_image
                .entry(image_id)
                .or_insert_with(Vec::new)
                .push(name);
        }

        Ok(tags_by_image)
    }

    fn normalize_tag(value: &str) -> Option<(String, String)> {
        let cleaned = value
            .trim()
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if cleaned.is_empty() {
            return None;
        }

        Some((cleaned.clone(), cleaned.to_lowercase()))
    }

    fn stored_image_from_row(
        row: StoredImageRow,
        tags: Vec<String>,
    ) -> Result<StoredImage, sqlx::Error> {
        let (
            stored_id,
            stored_owner_user_id,
            stored_pool_id,
            stored_url,
            stored_title,
            stored_favorite,
            stored_random_weight,
            created_at,
            stored_notes,
        ) = row;

        Ok(StoredImage {
            id: Uuid::parse_str(&stored_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            owner_user_id: Uuid::parse_str(&stored_owner_user_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            pool_id: Uuid::parse_str(&stored_pool_id)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            url: stored_url,
            title: stored_title,
            favorite: stored_favorite,
            random_weight: stored_random_weight,
            tags,
            created_at,
            notes: stored_notes,
        })
    }
}

fn normalized_search_query(query: &Option<String>) -> Option<String> {
    query
        .as_deref()
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(str::to_lowercase)
}

fn like_pattern(value: &str) -> String {
    let mut pattern = String::from("%");
    for character in value.chars() {
        match character {
            '\\' | '%' | '_' => {
                pattern.push('\\');
                pattern.push(character);
            }
            _ => pattern.push(character),
        }
    }
    pattern.push('%');
    pattern
}

fn normalized_filter_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for tag in tags {
        let Some((_, folded)) = ImageRepository::normalize_tag(tag) else {
            continue;
        };
        if seen.insert(folded.clone()) {
            normalized.push(folded);
        }
    }

    normalized
}

fn normalized_tag_names(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for tag in tags {
        let Some((name, folded)) = ImageRepository::normalize_tag(tag) else {
            continue;
        };
        if seen.insert(folded) {
            normalized.push(name);
        }
    }

    normalized
}

fn normalized_tag_folds(tags: &[String]) -> HashSet<String> {
    tags.iter()
        .filter_map(|tag| ImageRepository::normalize_tag(tag).map(|(_, folded)| folded))
        .collect()
}
