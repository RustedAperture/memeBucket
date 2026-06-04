use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState, auth::sessions::AuthenticatedUser, error::AppError,
    repositories::categories::CategoryRepository,
};

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
}

pub async fn list_categories(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let categories = CategoryRepository::new(state.pool)
        .list_for_user(user.user_id)
        .await?;

    Ok(Json(
        categories
            .into_iter()
            .map(|category| CategoryResponse {
                id: category.id.to_string(),
                name: category.name,
            })
            .collect(),
    ))
}

pub async fn create_category(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "category name is required".to_string(),
        ));
    }

    let category = match CategoryRepository::new(state.pool)
        .create(user.user_id, &request.name)
        .await
    {
        Ok(category) => category,
        Err(sqlx::Error::RowNotFound) => {
            return Err(AppError::BadRequest("category already exists".to_string()));
        }
        Err(err) => return Err(err.into()),
    };

    Ok(Json(CategoryResponse {
        id: category.id.to_string(),
        name: category.name,
    }))
}

pub async fn delete_category(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(category_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = CategoryRepository::new(state.pool)
        .delete_for_user(user.user_id, category_id)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
