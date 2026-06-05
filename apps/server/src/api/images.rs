use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::sessions::AuthenticatedUser,
    error::AppError,
    repositories::images::ImageRepository,
};

#[derive(Deserialize)]
pub struct CreateImageRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ImageResponse {
    pub id: String,
    pub url: String,
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
    if request.url.trim().is_empty() {
        return Err(AppError::BadRequest("url is required".to_string()));
    }
    let repo = ImageRepository::new(state.pool);
    let image = repo
        .create(user.user_id, pool_id, request.url.trim())
        .await?;
    Ok(Json(ImageResponse {
        id: image.id.to_string(),
        url: image.url,
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
