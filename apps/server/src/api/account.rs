use axum::{Json, extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse};

use crate::{
    app_state::AppState, 
    auth::sessions::{AuthenticatedUser, read_session_cookie, delete_session, expired_session_cookie}, 
    services::account::AccountService,
    repositories::users::UserRepository,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, crate::error::AppError> {
    if let Some(session_id) = read_session_cookie(&headers) {
        let _ = delete_session(&state.pool, &session_id).await;
    }
    
    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, expired_session_cookie())],
        Json(serde_json::json!({ "logged_out": true })),
    ))
}

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

pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserProfileResponse>, crate::error::AppError> {
    let repo = UserRepository::new(state.pool.clone());
    let stored = repo.get_by_id(user.user_id).await?.ok_or_else(|| {
        crate::error::AppError::NotFound
    })?;

    Ok(Json(UserProfileResponse {
        id: stored.id.to_string(),
        username: stored.username,
        display_name: stored.display_name,
        avatar_url: stored.avatar_url,
    }))
}

pub async fn update_username(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<UpdateUsernameRequest>,
) -> Result<Json<UserProfileResponse>, crate::error::AppError> {
    let username = req.username.trim();
    if username.is_empty() || username.len() < 3 || username.len() > 32 {
        return Err(crate::error::AppError::BadRequest("Username must be between 3 and 32 characters".into()));
    }
    // simple regex validation: alphanumeric and underscores only
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(crate::error::AppError::BadRequest("Username can only contain letters, numbers, and underscores".into()));
    }

    let repo = UserRepository::new(state.pool.clone());
    
    // SQLite UNIQUE constraint will catch duplicates, but we could return a nice error.
    match repo.update_username(user.user_id, username).await {
        Ok(_) => (),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(crate::error::AppError::BadRequest("Username is already taken".into()));
        }
        Err(e) => return Err(e.into()),
    }

    get_profile(State(state), user).await
}
