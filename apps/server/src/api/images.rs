use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::sessions::AuthenticatedUser,
    error::AppError,
    repositories::{
        images::{ImageRepository, StoredImage, UpdateImageMetadataPatch},
        send_history::SendHistoryRepository,
    },
    services::images::resolve_image_url,
};

#[derive(Deserialize)]
pub struct CreateImageRequest {
    pub url: String,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ImageResponse {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub favorite: bool,
    #[serde(rename = "randomWeight")]
    pub random_weight: i64,
    pub tags: Vec<String>,
    #[serde(rename = "sendCount")]
    pub send_count: i64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateImageRequest {
    #[serde(default, deserialize_with = "deserialize_optional_nullable")]
    pub notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_nullable")]
    pub title: Option<Option<String>>,
    pub favorite: Option<bool>,
    #[serde(rename = "randomWeight")]
    pub random_weight: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct MoveImageRequest {
    pub new_pool_id: Uuid,
}

pub async fn list_images(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(pool_id): Path<Uuid>,
) -> Result<Json<Vec<ImageResponse>>, AppError> {
    let repo = ImageRepository::new(state.pool.clone());
    let images = repo.list_for_pool(user.user_id, pool_id).await?;
    Ok(Json(
        build_image_responses(state.pool, Some(user.user_id), images).await?,
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
    let title = validate_title(request.title)?;
    let mut resolved_url = resolve_image_url(url)
        .await
        .map_err(|err| AppError::BadRequest(err.user_message().to_string()))?;

    let is_video = {
        let base = resolved_url
            .split('?')
            .next()
            .unwrap_or(&resolved_url)
            .to_lowercase();
        base.ends_with(".mp4") || base.ends_with(".webm")
    };

    if is_video && let Some(key) = &state.imgbb_api_key {
        resolved_url = crate::services::video_converter::convert_and_upload_mp4(&resolved_url, key)
            .await
            .map_err(|err| {
                AppError::InternalServerError(format!("Failed to convert video to GIF: {}", err))
            })?;
    }

    let repo = ImageRepository::new(state.pool);
    let image = repo
        .create_with_metadata(
            user.user_id,
            pool_id,
            &resolved_url,
            title.as_deref(),
            false,
            1,
            &request.tags.unwrap_or_default(),
        )
        .await?;
    Ok(Json(image_response_from_stored(image, 0)))
}

pub async fn delete_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((pool_id, image_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ImageRepository::new(state.pool);
    let deleted = repo
        .delete_for_user(user.user_id, pool_id, image_id)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub async fn update_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((pool_id, image_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ImageRepository::new(state.pool.clone());
    repo.get_for_owner(user.user_id, pool_id, image_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let patch = UpdateImageMetadataPatch {
        title: match request.title {
            Some(title) => Some(validate_title(title)?),
            None => None,
        },
        notes: request.notes.map(normalize_optional_text),
        favorite: request.favorite,
        random_weight: request
            .random_weight
            .map(validate_random_weight)
            .transpose()?,
        tags: request.tags,
    };

    let updated = repo
        .update_metadata_partial(user.user_id, pool_id, image_id, &patch)
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "updated": true })))
}

pub async fn move_image(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((pool_id, image_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ImageRepository::new(state.pool);
    let updated = repo
        .move_to_pool(user.user_id, pool_id, image_id, request.new_pool_id)
        .await?;

    if !updated {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({ "updated": true })))
}

pub async fn build_image_responses(
    pool: SqlitePool,
    requester_user_id: Option<Uuid>,
    images: Vec<StoredImage>,
) -> Result<Vec<ImageResponse>, sqlx::Error> {
    let send_counts = match requester_user_id {
        Some(requester_user_id) => {
            let send_history = SendHistoryRepository::new(pool);
            let image_ids = images.iter().map(|image| image.id).collect::<Vec<_>>();
            send_history
                .count_for_images(requester_user_id, &image_ids)
                .await?
        }
        None => std::collections::HashMap::new(),
    };

    Ok(images
        .into_iter()
        .map(|image| {
            let send_count = send_counts.get(&image.id).copied().unwrap_or(0);
            image_response_from_stored(image, send_count)
        })
        .collect())
}

pub fn image_response_from_stored(image: StoredImage, send_count: i64) -> ImageResponse {
    ImageResponse {
        id: image.id.to_string(),
        url: image.url,
        title: image.title,
        favorite: image.favorite,
        random_weight: image.random_weight,
        tags: image.tags,
        send_count,
        created_at: image.created_at,
        notes: image.notes,
    }
}

fn validate_title(title: Option<String>) -> Result<Option<String>, AppError> {
    match title {
        Some(title) if title.chars().count() > 200 => Err(AppError::BadRequest(
            "title must be 200 characters or fewer".to_string(),
        )),
        Some(title) => Ok(normalize_optional_text(Some(title))),
        None => Ok(None),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    match value {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(value),
        None => None,
    }
}

fn deserialize_optional_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

fn validate_random_weight(random_weight: i64) -> Result<i64, AppError> {
    if (0..=10).contains(&random_weight) {
        Ok(random_weight)
    } else {
        Err(AppError::BadRequest(
            "randomWeight must be between 0 and 10".to_string(),
        ))
    }
}
