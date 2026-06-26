use std::time::Duration;
use moka::future::Cache;
use uuid::Uuid;

use crate::repositories::{
    buckets::{BucketRepo, StoredBucket},
    images::{ImageRepo, StoredImage, StoredImageSearchResult, ImageSearchFilters, UpdateImageMetadataPatch, BulkImageMetadataPatch},
};

#[derive(Clone)]
pub struct CachedBucketRepository<R: BucketRepo> {
    inner: R,
    user_buckets: Cache<Uuid, Vec<StoredBucket>>,
    user_bucket_names: Cache<Uuid, Vec<String>>,
    bucket_by_id: Cache<Uuid, Option<StoredBucket>>,
    bucket_by_token: Cache<String, Option<StoredBucket>>,
    user_subscribed: Cache<Uuid, Vec<StoredBucket>>,
    whitelist_users: Cache<Uuid, Option<Vec<String>>>,
    is_whitelisted: Cache<(Uuid, Uuid), bool>,
}

impl<R: BucketRepo> CachedBucketRepository<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            user_buckets: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            user_bucket_names: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            bucket_by_id: Cache::builder()
                .max_capacity(2000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            bucket_by_token: Cache::builder()
                .max_capacity(2000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            user_subscribed: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            whitelist_users: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            is_whitelisted: Cache::builder()
                .max_capacity(5000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    async fn invalidate_for_user(&self, user_id: Uuid) {
        self.user_buckets.invalidate(&user_id).await;
        self.user_bucket_names.invalidate(&user_id).await;
    }

    async fn invalidate_bucket(&self, bucket_id: Uuid) {
        self.bucket_by_id.invalidate(&bucket_id).await;
        self.whitelist_users.invalidate(&bucket_id).await;
        self.user_subscribed.invalidate_all();
        self.is_whitelisted.invalidate_all();
    }

    async fn invalidate_bucket_with_token(&self, bucket_id: Uuid) {
        if let Ok(Some(bucket)) = self.inner.get_by_id(bucket_id).await {
            if let Some(token) = &bucket.share_token {
                self.bucket_by_token.invalidate(token).await;
            }
            self.invalidate_for_user(bucket.owner_user_id).await;
        }
        self.invalidate_bucket(bucket_id).await;
    }
}

#[async_trait::async_trait]
impl<R: BucketRepo> BucketRepo for CachedBucketRepository<R> {
    async fn create(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<StoredBucket, sqlx::Error> {
        let bucket = self.inner.create(owner_user_id, name).await?;
        self.invalidate_for_user(owner_user_id).await;
        Ok(bucket)
    }

    async fn rename_bucket(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        new_name: &str,
    ) -> Result<bool, sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.rename_bucket(bucket_id, owner_user_id, new_name).await?;
        self.invalidate_bucket_with_token(bucket_id).await;
        Ok(result)
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StoredBucket>, sqlx::Error> {
        if let Some(cached) = self.user_buckets.get(&user_id).await {
            return Ok(cached);
        }
        let result = self.inner.list_for_user(user_id).await?;
        self.user_buckets.insert(user_id, result.clone()).await;
        Ok(result)
    }

    async fn list_bucket_names_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        if let Some(cached) = self.user_bucket_names.get(&user_id).await {
            return Ok(cached);
        }
        let result = self.inner.list_bucket_names_for_user(user_id).await?;
        self.user_bucket_names.insert(user_id, result.clone()).await;
        Ok(result)
    }

    async fn delete_for_user(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.delete_for_user(user_id, bucket_id).await?;
        self.invalidate_bucket_with_token(bucket_id).await;
        Ok(result)
    }

    async fn find_by_name_folded(
        &self,
        owner_user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        self.inner.find_by_name_folded(owner_user_id, name).await
    }

    async fn find_accessible_by_name_folded(
        &self,
        requester_user_id: Uuid,
        name: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        self.inner.find_accessible_by_name_folded(requester_user_id, name).await
    }

    async fn get_by_id(&self, bucket_id: Uuid) -> Result<Option<StoredBucket>, sqlx::Error> {
        if let Some(cached) = self.bucket_by_id.get(&bucket_id).await {
            return Ok(cached);
        }
        let result = self.inner.get_by_id(bucket_id).await?;
        self.bucket_by_id.insert(bucket_id, result.clone()).await;
        Ok(result)
    }

    async fn get_by_share_token(
        &self,
        share_token: &str,
    ) -> Result<Option<StoredBucket>, sqlx::Error> {
        let token_str = share_token.to_string();
        if let Some(cached) = self.bucket_by_token.get(&token_str).await {
            return Ok(cached);
        }
        let result = self.inner.get_by_share_token(share_token).await?;
        self.bucket_by_token.insert(token_str, result.clone()).await;
        Ok(result)
    }

    async fn set_share_token(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        token: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.set_share_token(bucket_id, owner_user_id, token).await?;
        self.invalidate_bucket_with_token(bucket_id).await;
        Ok(result)
    }

    async fn subscribe_user_to_bucket(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.subscribe_user_to_bucket(user_id, bucket_id).await?;
        self.user_subscribed.invalidate(&user_id).await;
        Ok(result)
    }

    async fn unsubscribe_user_from_bucket(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.unsubscribe_user_from_bucket(user_id, bucket_id).await?;
        self.user_subscribed.invalidate(&user_id).await;
        Ok(result)
    }

    async fn list_subscribed_for_user(
        &self,
        subscriber_user_id: Uuid,
    ) -> Result<Vec<StoredBucket>, sqlx::Error> {
        if let Some(cached) = self.user_subscribed.get(&subscriber_user_id).await {
            return Ok(cached);
        }
        let result = self.inner.list_subscribed_for_user(subscriber_user_id).await?;
        self.user_subscribed.insert(subscriber_user_id, result.clone()).await;
        Ok(result)
    }

    async fn set_whitelist_enabled(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        enabled: bool,
    ) -> Result<bool, sqlx::Error> {
        self.invalidate_bucket_with_token(bucket_id).await;
        let result = self.inner.set_whitelist_enabled(bucket_id, owner_user_id, enabled).await?;
        self.invalidate_bucket_with_token(bucket_id).await;
        Ok(result)
    }

    async fn add_whitelist_user(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.add_whitelist_user(bucket_id, owner_user_id, username).await?;
        self.whitelist_users.invalidate(&bucket_id).await;
        self.is_whitelisted.invalidate_all();
        Ok(result)
    }

    async fn remove_whitelist_user(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.remove_whitelist_user(bucket_id, owner_user_id, username).await?;
        self.whitelist_users.invalidate(&bucket_id).await;
        self.is_whitelisted.invalidate_all();
        Ok(result)
    }

    async fn list_whitelist_users(
        &self,
        bucket_id: Uuid,
        owner_user_id: Uuid,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        if let Some(cached) = self.whitelist_users.get(&bucket_id).await {
            return Ok(cached);
        }
        let result = self.inner.list_whitelist_users(bucket_id, owner_user_id).await?;
        self.whitelist_users.insert(bucket_id, result.clone()).await;
        Ok(result)
    }

    async fn is_user_whitelisted(
        &self,
        bucket_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let key = (bucket_id, user_id);
        if let Some(cached) = self.is_whitelisted.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.is_user_whitelisted(bucket_id, user_id).await?;
        self.is_whitelisted.insert(key, result).await;
        Ok(result)
    }
}

#[derive(Clone)]
pub struct CachedImageRepository<R: ImageRepo> {
    inner: R,
    list_for_bucket: Cache<(Uuid, Uuid), Vec<StoredImage>>,
    get_for_owner: Cache<(Uuid, Uuid, Uuid), Option<StoredImage>>,
}

impl<R: ImageRepo> CachedImageRepository<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            list_for_bucket: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            get_for_owner: Cache::builder()
                .max_capacity(5000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }
}

#[async_trait::async_trait]
impl<R: ImageRepo> ImageRepo for CachedImageRepository<R> {
    async fn create(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        url: &str,
    ) -> Result<StoredImage, sqlx::Error> {
        let image = self.inner.create(owner_user_id, bucket_id, url).await?;
        self.list_for_bucket.invalidate_all();
        Ok(image)
    }

    async fn create_with_metadata(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        url: &str,
        title: Option<&str>,
        favorite: bool,
        random_weight: i64,
        tags: &[String],
    ) -> Result<StoredImage, sqlx::Error> {
        let image = self.inner.create_with_metadata(
            owner_user_id,
            bucket_id,
            url,
            title,
            favorite,
            random_weight,
            tags,
        ).await?;
        self.list_for_bucket.invalidate_all();
        Ok(image)
    }

    async fn list_for_bucket(
        &self,
        user_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<Vec<StoredImage>, sqlx::Error> {
        let key = (user_id, bucket_id);
        if let Some(cached) = self.list_for_bucket.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.list_for_bucket(user_id, bucket_id).await?;
        self.list_for_bucket.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn search_for_user(
        &self,
        owner_user_id: Uuid,
        filters: &ImageSearchFilters,
    ) -> Result<Vec<StoredImageSearchResult>, sqlx::Error> {
        self.inner.search_for_user(owner_user_id, filters).await
    }

    async fn get_for_owner(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
    ) -> Result<Option<StoredImage>, sqlx::Error> {
        let key = (owner_user_id, bucket_id, image_id);
        if let Some(cached) = self.get_for_owner.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.get_for_owner(owner_user_id, bucket_id, image_id).await?;
        self.get_for_owner.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn delete_for_user(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.delete_for_user(owner_user_id, bucket_id, image_id).await?;
        self.list_for_bucket.invalidate_all();
        self.get_for_owner.invalidate(&(owner_user_id, bucket_id, image_id)).await;
        Ok(result)
    }

    async fn update_notes(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
        notes: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.update_notes(owner_user_id, bucket_id, image_id, notes).await?;
        self.list_for_bucket.invalidate_all();
        self.get_for_owner.invalidate(&(owner_user_id, bucket_id, image_id)).await;
        Ok(result)
    }

    async fn update_metadata(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
        title: Option<&str>,
        notes: Option<&str>,
        favorite: bool,
        random_weight: i64,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.update_metadata(
            owner_user_id,
            bucket_id,
            image_id,
            title,
            notes,
            favorite,
            random_weight,
            tags,
        ).await?;
        self.list_for_bucket.invalidate_all();
        self.get_for_owner.invalidate(&(owner_user_id, bucket_id, image_id)).await;
        Ok(result)
    }

    async fn update_metadata_partial(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
        patch: &UpdateImageMetadataPatch,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.update_metadata_partial(owner_user_id, bucket_id, image_id, patch).await?;
        self.list_for_bucket.invalidate_all();
        self.get_for_owner.invalidate(&(owner_user_id, bucket_id, image_id)).await;
        Ok(result)
    }

    async fn update_metadata_bulk(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        patch: &BulkImageMetadataPatch,
    ) -> Result<usize, sqlx::Error> {
        let result = self.inner.update_metadata_bulk(owner_user_id, bucket_id, patch).await?;
        self.list_for_bucket.invalidate_all();
        for id in &patch.image_ids {
            self.get_for_owner.invalidate(&(owner_user_id, bucket_id, *id)).await;
        }
        Ok(result)
    }

    async fn delete_bulk(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_ids: &[Uuid],
    ) -> Result<usize, sqlx::Error> {
        let result = self.inner.delete_bulk(owner_user_id, bucket_id, image_ids).await?;
        self.list_for_bucket.invalidate_all();
        for id in image_ids {
            self.get_for_owner.invalidate(&(owner_user_id, bucket_id, *id)).await;
        }
        Ok(result)
    }

    async fn move_bulk(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_ids: &[Uuid],
        new_bucket_id: Uuid,
    ) -> Result<usize, sqlx::Error> {
        let result = self.inner.move_bulk(owner_user_id, bucket_id, image_ids, new_bucket_id).await?;
        self.list_for_bucket.invalidate_all();
        for id in image_ids {
            self.get_for_owner.invalidate(&(owner_user_id, bucket_id, *id)).await;
            self.get_for_owner.invalidate(&(owner_user_id, new_bucket_id, *id)).await;
        }
        Ok(result)
    }

    async fn move_to_bucket(
        &self,
        owner_user_id: Uuid,
        bucket_id: Uuid,
        image_id: Uuid,
        new_bucket_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = self.inner.move_to_bucket(owner_user_id, bucket_id, image_id, new_bucket_id).await?;
        self.list_for_bucket.invalidate_all();
        self.get_for_owner.invalidate(&(owner_user_id, bucket_id, image_id)).await;
        self.get_for_owner.invalidate(&(owner_user_id, new_bucket_id, image_id)).await;
        Ok(result)
    }
}
