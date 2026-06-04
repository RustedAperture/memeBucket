use axum::Json;
use serde::Serialize;

use crate::auth::sessions::AuthenticatedUser;

#[derive(Serialize)]
pub struct CategoryListResponse {
    pub categories: Vec<serde_json::Value>,
}

pub async fn list_categories(_user: AuthenticatedUser) -> Json<CategoryListResponse> {
    Json(CategoryListResponse {
        categories: vec![],
    })
}
