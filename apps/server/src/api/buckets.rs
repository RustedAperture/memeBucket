use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::middleware::OptionalUser,
    auth::sessions::AuthenticatedUser,
    error::AppError,
    repositories::buckets::{BucketRepository, StoredBucket},
};

#[derive(Deserialize)]
pub struct CreateBucketRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct BucketResponse {
    pub id: String,
    pub name: String,
    pub share_token: Option<String>,
    pub subscriber_count: i64,
    pub is_subscribed: bool,
    pub owner_username: Option<String>,
    pub whitelist_enabled: bool,
    pub image_count: i64,
    pub is_read_only: bool,
}

impl From<StoredBucket> for BucketResponse {
    fn from(bucket: StoredBucket) -> Self {
        Self {
            id: bucket.id.to_string(),
            name: bucket.name,
            share_token: bucket.share_token,
            subscriber_count: bucket.subscriber_count,
            is_subscribed: false,
            owner_username: bucket.owner_username,
            whitelist_enabled: bucket.whitelist_enabled,
            image_count: bucket.image_count,
            is_read_only: false,
        }
    }
}

pub async fn list_buckets(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<BucketResponse>>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let owned = repo.list_for_user(user.user_id).await?;
    let subscribed = repo.list_subscribed_for_user(user.user_id).await?;

    let mut response = Vec::new();
    for bucket in owned {
        let mut br = BucketResponse::from(bucket);
        br.is_subscribed = false;
        br.is_read_only = br.name == "Added from Discord";

        if br.is_read_only && br.image_count == 0 {
            continue;
        }

        response.push(br);
    }
    for bucket in subscribed {
        let mut br = BucketResponse::from(bucket);
        br.is_subscribed = true;
        br.is_read_only = br.name == "Added from Discord";

        if br.is_read_only && br.image_count == 0 {
            continue;
        }

        response.push(br);
    }

    response.sort_by_key(|a| a.name.to_lowercase());

    Ok(Json(response))
}

pub async fn create_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateBucketRequest>,
) -> Result<Json<BucketResponse>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest("bucket name is required".to_string()));
    }

    let bucket = match BucketRepository::new(state.pool)
        .create(user.user_id, &request.name)
        .await
    {
        Ok(bucket) => bucket,
        Err(sqlx::Error::RowNotFound) => {
            return Err(AppError::BadRequest("bucket already exists".to_string()));
        }
        Err(err) => return Err(err.into()),
    };

    Ok(Json(BucketResponse {
        id: bucket.id.to_string(),
        name: bucket.name,
        share_token: bucket.share_token,
        subscriber_count: bucket.subscriber_count,
        is_subscribed: false,
        owner_username: bucket.owner_username,
        whitelist_enabled: bucket.whitelist_enabled,
        image_count: 0,
        is_read_only: false,
    }))
}

pub async fn delete_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = BucketRepository::new(state.pool)
        .delete_for_user(user.user_id, bucket_id)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
pub struct RenameBucketRequest {
    pub name: String,
}

pub async fn rename_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
    Json(request): Json<RenameBucketRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest("bucket name is required".to_string()));
    }

    match BucketRepository::new(state.pool)
        .rename_bucket(bucket_id, user.user_id, &request.name)
        .await
    {
        Ok(true) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => {
            if e.as_database_error()
                .is_some_and(|db_err| db_err.is_unique_violation())
            {
                return Err(AppError::BadRequest("bucket already exists".to_string()));
            }
            Err(e.into())
        }
    }
}

use rand::Rng;

pub async fn share_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    let updated = BucketRepository::new(state.pool)
        .set_share_token(bucket_id, user.user_id, Some(&token))
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "share_token": token })))
}

pub async fn unshare_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let updated = BucketRepository::new(state.pool)
        .set_share_token(bucket_id, user.user_id, None)
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "unshared": true })))
}

pub async fn subscribe_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let bucket = repo
        .get_by_share_token(&token)
        .await?
        .ok_or(AppError::NotFound)?;

    if bucket.whitelist_enabled {
        let is_whitelisted = repo.is_user_whitelisted(bucket.id, user.user_id).await?;
        if !is_whitelisted {
            return Err(AppError::Forbidden);
        }
    }

    repo.subscribe_user_to_bucket(user.user_id, bucket.id)
        .await?;
    Ok(Json(serde_json::json!({ "subscribed": true })))
}

pub async fn unsubscribe_bucket(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let deleted = repo
        .unsubscribe_user_from_bucket(user.user_id, bucket_id)
        .await?;
    Ok(Json(serde_json::json!({ "unsubscribed": deleted })))
}

#[derive(Serialize)]
pub struct SharedBucketPreview {
    pub id: String,
    pub name: String,
    pub subscriber_count: i64,
    pub images: Vec<crate::api::images::ImageResponse>,
}

pub async fn get_shared_bucket(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Path(token): Path<String>,
) -> Result<Json<SharedBucketPreview>, AppError> {
    let repo = BucketRepository::new(state.pool.clone());
    let bucket = repo
        .get_by_share_token(&token)
        .await?
        .ok_or(AppError::NotFound)?;

    if bucket.whitelist_enabled {
        let is_allowed = match user {
            Some(ref u) => repo.is_user_whitelisted(bucket.id, u.user_id).await?,
            None => false,
        };
        if !is_allowed {
            return Err(AppError::Forbidden);
        }
    }

    let image_repo = crate::repositories::images::ImageRepository::new(state.pool.clone());
    let images = image_repo
        .list_for_bucket(bucket.owner_user_id, bucket.id)
        .await?;
    let requester_user_id = user.as_ref().map(|user| user.user_id);
    let image_responses =
        crate::api::images::build_image_responses(state.pool.clone(), requester_user_id, images)
            .await?;

    Ok(Json(SharedBucketPreview {
        id: bucket.id.to_string(),
        name: bucket.name,
        subscriber_count: bucket.subscriber_count,
        images: image_responses,
    }))
}

#[derive(Deserialize)]
pub struct WhitelistEnabledRequest {
    pub enabled: bool,
}

pub async fn set_whitelist_enabled(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
    Json(req): Json<WhitelistEnabledRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let updated = repo
        .set_whitelist_enabled(bucket_id, user.user_id, req.enabled)
        .await?;
    if !updated {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct AddWhitelistUserRequest {
    pub username: String,
}

pub async fn add_whitelist_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
    Json(req): Json<AddWhitelistUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let added = repo
        .add_whitelist_user(bucket_id, user.user_id, &req.username)
        .await?;
    if !added {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn remove_whitelist_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((bucket_id, username)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let removed = repo
        .remove_whitelist_user(bucket_id, user.user_id, &username)
        .await?;
    if !removed {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn list_whitelist_users(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(bucket_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    let repo = BucketRepository::new(state.pool);
    let users = repo
        .list_whitelist_users(bucket_id, user.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(users))
}
