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
    repositories::pools::{PoolRepository, StoredPool},
};

#[derive(Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct PoolResponse {
    pub id: String,
    pub name: String,
    pub share_token: Option<String>,
    pub subscriber_count: i64,
    pub is_subscribed: bool,
    pub owner_username: Option<String>,
    pub whitelist_enabled: bool,
}

impl From<StoredPool> for PoolResponse {
    fn from(pool: StoredPool) -> Self {
        Self {
            id: pool.id.to_string(),
            name: pool.name,
            share_token: pool.share_token,
            subscriber_count: pool.subscriber_count,
            is_subscribed: false,
            owner_username: pool.owner_username,
            whitelist_enabled: pool.whitelist_enabled,
        }
    }
}

pub async fn list_pools(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<PoolResponse>>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let owned = repo.list_for_user(user.user_id).await?;
    let subscribed = repo.list_subscribed_for_user(user.user_id).await?;

    let mut response = Vec::new();
    for pool in owned {
        let mut pr = PoolResponse::from(pool);
        pr.is_subscribed = false;
        response.push(pr);
    }
    for pool in subscribed {
        let mut pr = PoolResponse::from(pool);
        pr.is_subscribed = true;
        response.push(pr);
    }

    response.sort_by_key(|a| a.name.to_lowercase());

    Ok(Json(response))
}

pub async fn create_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreatePoolRequest>,
) -> Result<Json<PoolResponse>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest("pool name is required".to_string()));
    }

    let pool = match PoolRepository::new(state.pool)
        .create(user.user_id, &request.name)
        .await
    {
        Ok(pool) => pool,
        Err(sqlx::Error::RowNotFound) => {
            return Err(AppError::BadRequest("pool already exists".to_string()));
        }
        Err(err) => return Err(err.into()),
    };

    Ok(Json(PoolResponse {
        id: pool.id.to_string(),
        name: pool.name,
        share_token: pool.share_token,
        subscriber_count: pool.subscriber_count,
        is_subscribed: false,
        owner_username: pool.owner_username,
        whitelist_enabled: pool.whitelist_enabled,
    }))
}

pub async fn delete_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = PoolRepository::new(state.pool)
        .delete_for_user(user.user_id, pool_id)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
pub struct RenamePoolRequest {
    pub name: String,
}

pub async fn rename_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
    Json(request): Json<RenamePoolRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest("pool name is required".to_string()));
    }

    match PoolRepository::new(state.pool)
        .rename_pool(pool_id, user.user_id, &request.name)
        .await
    {
        Ok(true) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(false) => Err(AppError::NotFound),
        Err(e) => {
            if e.as_database_error()
                .is_some_and(|db_err| db_err.is_unique_violation())
            {
                return Err(AppError::BadRequest("pool already exists".to_string()));
            }
            Err(e.into())
        }
    }
}

use rand::Rng;

pub async fn share_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    let updated = PoolRepository::new(state.pool)
        .set_share_token(pool_id, user.user_id, Some(&token))
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "share_token": token })))
}

pub async fn unshare_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let updated = PoolRepository::new(state.pool)
        .set_share_token(pool_id, user.user_id, None)
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "unshared": true })))
}

pub async fn subscribe_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let pool = repo
        .get_by_share_token(&token)
        .await?
        .ok_or(AppError::NotFound)?;

    if pool.whitelist_enabled {
        let is_whitelisted = repo.is_user_whitelisted(pool.id, user.user_id).await?;
        if !is_whitelisted {
            return Err(AppError::Forbidden);
        }
    }

    repo.subscribe_user_to_pool(user.user_id, pool.id).await?;
    Ok(Json(serde_json::json!({ "subscribed": true })))
}

pub async fn unsubscribe_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let deleted = repo
        .unsubscribe_user_from_pool(user.user_id, pool_id)
        .await?;
    Ok(Json(serde_json::json!({ "unsubscribed": deleted })))
}

#[derive(Serialize)]
pub struct SharedPoolPreview {
    pub id: String,
    pub name: String,
    pub subscriber_count: i64,
    pub images: Vec<crate::api::images::ImageResponse>,
}

pub async fn get_shared_pool(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Path(token): Path<String>,
) -> Result<Json<SharedPoolPreview>, AppError> {
    let repo = PoolRepository::new(state.pool.clone());
    let pool = repo
        .get_by_share_token(&token)
        .await?
        .ok_or(AppError::NotFound)?;

    if pool.whitelist_enabled {
        let is_allowed = match user {
            Some(u) => repo.is_user_whitelisted(pool.id, u.user_id).await?,
            None => false,
        };
        if !is_allowed {
            return Err(AppError::Forbidden);
        }
    }

    let image_repo = crate::repositories::images::ImageRepository::new(state.pool);
    let images = image_repo
        .list_for_pool(pool.owner_user_id, pool.id)
        .await?;

    Ok(Json(SharedPoolPreview {
        id: pool.id.to_string(),
        name: pool.name,
        subscriber_count: pool.subscriber_count,
        images: images
            .into_iter()
            .map(|img| crate::api::images::ImageResponse {
                id: img.id.to_string(),
                url: img.url,
                created_at: img.created_at,
                notes: img.notes,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
pub struct WhitelistEnabledRequest {
    pub enabled: bool,
}

pub async fn set_whitelist_enabled(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
    Json(req): Json<WhitelistEnabledRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let updated = repo
        .set_whitelist_enabled(pool_id, user.user_id, req.enabled)
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
    Path(pool_id): Path<Uuid>,
    Json(req): Json<AddWhitelistUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let added = repo
        .add_whitelist_user(pool_id, user.user_id, &req.username)
        .await?;
    if !added {
        return Err(AppError::NotFound); // Could mean pool not found/owned or user not found
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn remove_whitelist_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((pool_id, username)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let removed = repo
        .remove_whitelist_user(pool_id, user.user_id, &username)
        .await?;
    if !removed {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn list_whitelist_users(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    let repo = PoolRepository::new(state.pool);
    let users = repo
        .list_whitelist_users(pool_id, user.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(users))
}
