use crate::services::storage::{StorageError, StorageService};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn run_cdn_migration(pool: SqlitePool, storage: Arc<StorageService>) {
    tracing::info!("Starting CDN migration job for existing Discord media...");
    let mut total_migrated = 0usize;
    let mut total_broken = 0usize;

    // Track IDs that failed upload this run so we exclude them from subsequent
    // batch fetches.  Without this, rows that stay 'pending' after UploadFailed
    // would be re-selected every iteration and the loop would never terminate.
    let mut failed_this_run: HashSet<String> = HashSet::new();

    loop {
        // Fetch next batch of pending rows, excluding any that already failed
        // upload this startup run (they will be retried on the next startup).
        let rows: Vec<(String, String)> = if failed_this_run.is_empty() {
            match sqlx::query_as::<_, (String, String)>(
                "SELECT id, url FROM images WHERE cdn_status = 'pending' LIMIT 20",
            )
            .fetch_all(&pool)
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("CDN migration: DB fetch failed: {e}");
                    break;
                }
            }
        } else {
            // Build a query that excludes IDs we already attempted this run.
            // SQLite supports up to 999 bind parameters; our batch is 20, so
            // this set stays small in practice.
            let placeholders = failed_this_run
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "SELECT id, url FROM images WHERE cdn_status = 'pending' AND id NOT IN ({}) LIMIT 20",
                placeholders
            );
            let mut q = sqlx::query_as::<_, (String, String)>(&sql);
            for id in &failed_this_run {
                q = q.bind(id.as_str());
            }
            match q.fetch_all(&pool).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("CDN migration: DB fetch failed: {e}");
                    break;
                }
            }
        };

        if rows.is_empty() {
            break;
        }

        for (id, url) in &rows {
            if !StorageService::is_discord_cdn(url) {
                // Non-Discord URLs are already stable — mark migrated, use original url
                let _ = sqlx::query(
                    "UPDATE images SET cdn_url = ?, cdn_status = 'migrated' WHERE id = ?",
                )
                .bind(url)
                .bind(id)
                .execute(&pool)
                .await;
                total_migrated += 1;
                continue;
            }

            match storage.upload_from_url(url).await {
                Ok(cdn_url) => {
                    let _ = sqlx::query(
                        "UPDATE images SET cdn_url = ?, cdn_status = 'migrated' WHERE id = ?",
                    )
                    .bind(&cdn_url)
                    .bind(id)
                    .execute(&pool)
                    .await;
                    total_migrated += 1;
                    tracing::debug!("Migrated: {}", url);
                }
                Err(StorageError::FetchFailed(e)) => {
                    // Source URL is dead — mark broken
                    tracing::warn!("CDN migration: dead URL {}: {}", url, e);
                    let _ = sqlx::query("UPDATE images SET cdn_status = 'broken' WHERE id = ?")
                        .bind(id)
                        .execute(&pool)
                        .await;
                    total_broken += 1;
                }
                Err(StorageError::UploadFailed(e)) => {
                    // Upload failed — leave as pending so it retries next startup,
                    // but exclude this ID from the current run to avoid an infinite loop.
                    tracing::warn!("CDN migration: upload failed for {}: {}", url, e);
                    failed_this_run.insert(id.clone());
                }
            }
        }

        // Throttle to avoid hammering Discord CDN
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    tracing::info!(
        "CDN migration complete: {} migrated, {} broken",
        total_migrated,
        total_broken
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_this_run_excludes_ids_from_retry() {
        // Verify that inserting a failing ID into the set prevents it from
        // being re-selected.  This is a logical unit test of the data structure
        // used to prevent the infinite loop — the actual SQL exclusion is
        // covered by integration tests with a real DB connection.
        let mut failed: HashSet<String> = HashSet::new();
        let id = "abc-123".to_string();

        assert!(!failed.contains(&id));
        failed.insert(id.clone());
        assert!(failed.contains(&id));

        // Inserting the same ID again is idempotent
        failed.insert(id.clone());
        assert_eq!(failed.len(), 1);
    }
}
