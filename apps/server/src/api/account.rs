use axum::{Json, extract::State};

use crate::{
    app_state::AppState, auth::sessions::AuthenticatedUser, services::account::AccountService,
};

pub async fn export_account(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<crate::services::account::ExportedUserData>, crate::error::AppError> {
    let service = AccountService::new(state.pool);
    Ok(Json(service.export_user_data(user.user_id).await?))
}

pub async fn delete_account(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let service = AccountService::new(state.pool);
    service.delete_account(user.user_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
