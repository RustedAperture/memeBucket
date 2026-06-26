use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repositories::{BucketRepo, ImageRepo, UserRepo};

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
    user_repo: Arc<dyn UserRepo>,
    bucket_repo: Arc<dyn BucketRepo>,
    image_repo: Arc<dyn ImageRepo>,
}

impl AccountService {
    pub fn new(
        user_repo: Arc<dyn UserRepo>,
        bucket_repo: Arc<dyn BucketRepo>,
        image_repo: Arc<dyn ImageRepo>,
    ) -> Self {
        Self {
            user_repo,
            bucket_repo,
            image_repo,
        }
    }

    pub async fn export_user_data(&self, user_id: Uuid) -> Result<ExportedUserData, sqlx::Error> {
        let bucket_rows = self.bucket_repo.list_for_user(user_id).await?;

        let mut buckets = Vec::with_capacity(bucket_rows.len());
        for bucket in bucket_rows {
            let image_rows = self.image_repo.list_for_bucket(user_id, bucket.id).await?;

            let mut images = Vec::with_capacity(image_rows.len());
            for img in image_rows {
                images.push(ExportedImage {
                    url: img.url,
                    title: img.title,
                    favorite: img.favorite,
                    random_weight: img.random_weight,
                    notes: img.notes,
                    tags: img.tags,
                    created_at: img.created_at,
                });
            }

            buckets.push(ExportedBucket {
                name: bucket.name,
                images,
            });
        }

        Ok(ExportedUserData { buckets })
    }

    pub async fn import_user_data(
        &self,
        user_id: Uuid,
        data: ExportedUserData,
    ) -> Result<(usize, usize), sqlx::Error> {
        self.user_repo.import_user_data(user_id, data).await
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        self.user_repo.delete(user_id).await
    }
}
