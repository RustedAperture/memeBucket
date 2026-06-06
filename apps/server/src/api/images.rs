use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState, auth::sessions::AuthenticatedUser, error::AppError,
    repositories::images::ImageRepository, services::images::resolve_image_url,
};

#[derive(Deserialize)]
pub struct CreateImageRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ImageResponse {
    pub id: String,
    pub url: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateImageRequest {
    pub notes: Option<String>,
}

pub async fn list_images(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<Vec<ImageResponse>>, AppError> {
    let repo = ImageRepository::new(state.pool);
    let images = repo.list_for_pool(user.user_id, pool_id).await?;
    Ok(Json(
        images
            .into_iter()
            .map(|image| ImageResponse {
                id: image.id.to_string(),
                url: image.url,
                created_at: image.created_at,
                notes: image.notes,
            })
            .collect(),
    ))
}

pub async fn create_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
    Json(request): Json<CreateImageRequest>,
) -> Result<Json<ImageResponse>, AppError> {
    let url = request.url.trim();
    if url.is_empty() {
        return Err(AppError::BadRequest("url is required".to_string()));
    }
    let resolved_url = resolve_image_url(url)
        .await
        .map_err(|err| AppError::BadRequest(err.user_message().to_string()))?;

    let repo = ImageRepository::new(state.pool);
    let image = repo.create(user.user_id, pool_id, &resolved_url).await?;
    Ok(Json(ImageResponse {
        id: image.id.to_string(),
        url: image.url,
        created_at: image.created_at,
        notes: image.notes,
    }))
}

pub async fn delete_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((_pool_id, image_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ImageRepository::new(state.pool);
    let deleted = repo.delete_for_user(user.user_id, image_id).await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub async fn update_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((_pool_id, image_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ImageRepository::new(state.pool);
    let updated = repo
        .update_notes(user.user_id, image_id, request.notes.as_deref())
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "updated": true })))
}
