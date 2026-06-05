use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState, auth::sessions::AuthenticatedUser, error::AppError,
    repositories::pools::PoolRepository,
};

#[derive(Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct PoolResponse {
    pub id: String,
    pub name: String,
}

pub async fn list_pools(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<PoolResponse>>, AppError> {
    let pools = PoolRepository::new(state.pool)
        .list_for_user(user.user_id)
        .await?;

    Ok(Json(
        pools
            .into_iter()
            .map(|pool| PoolResponse {
                id: pool.id.to_string(),
                name: pool.name,
            })
            .collect(),
    ))
}

pub async fn create_pool(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreatePoolRequest>,
) -> Result<Json<PoolResponse>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "pool name is required".to_string(),
        ));
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
