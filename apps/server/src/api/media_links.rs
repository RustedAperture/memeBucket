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
    repositories::media_links::MediaLinkRepository,
};

#[derive(Deserialize)]
pub struct CreateLinkRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct MediaLinkResponse {
    pub id: String,
    pub url: String,
}

pub async fn list_links(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(category_id): Path<Uuid>,
) -> Result<Json<Vec<MediaLinkResponse>>, AppError> {
    let repo = MediaLinkRepository::new(state.pool);
    let links = repo.list_for_category(user.user_id, category_id).await?;
    Ok(Json(
        links
            .into_iter()
            .map(|link| MediaLinkResponse {
                id: link.id.to_string(),
                url: link.url,
            })
            .collect(),
    ))
}

pub async fn create_link(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(category_id): Path<Uuid>,
    Json(request): Json<CreateLinkRequest>,
) -> Result<Json<MediaLinkResponse>, AppError> {
    if request.url.trim().is_empty() {
        return Err(AppError::BadRequest("url is required".to_string()));
    }
    let repo = MediaLinkRepository::new(state.pool);
    let link = repo
        .create(user.user_id, category_id, request.url.trim())
        .await?;
    Ok(Json(MediaLinkResponse {
        id: link.id.to_string(),
        url: link.url,
    }))
}

pub async fn delete_link(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((_category_id, link_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = MediaLinkRepository::new(state.pool);
    let deleted = repo.delete_for_user(user.user_id, link_id).await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
