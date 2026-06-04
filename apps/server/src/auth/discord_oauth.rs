use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn start_discord_oauth() -> impl IntoResponse {
    let url = std::env::var("DISCORD_OAUTH_AUTHORIZE_URL").unwrap_or_else(|_| {
        "https://discord.com/oauth2/authorize?scope=identify%20applications.commands".to_string()
    });
    (StatusCode::TEMPORARY_REDIRECT, Redirect::temporary(&url))
}

pub async fn handle_discord_oauth_callback(
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    if query.code.trim().is_empty() || query.state.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    Redirect::temporary("/").into_response()
}
