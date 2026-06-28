use crate::services::storage::{StorageError, StorageService};
use sqlx::SqlitePool;
use std::sync::Arc;

pub async fn run_cdn_migration(pool: SqlitePool, storage: Arc<StorageService>) {
    tracing::info!("Starting CDN migration job for existing Discord media...");
    let mut total_migrated = 0usize;
    let mut total_broken = 0usize;

    loop {
        // Fetch next batch of pending rows
        let rows: Vec<(String, String)> = match sqlx::query_as::<_, (String, String)>(
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
                    // Upload failed — leave as pending so it retries next startup
                    tracing::warn!("CDN migration: upload failed for {}: {}", url, e);
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
