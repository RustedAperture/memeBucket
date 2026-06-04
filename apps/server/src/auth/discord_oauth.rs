use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

pub async fn start_discord_oauth() -> impl IntoResponse {
    let url = std::env::var("DISCORD_OAUTH_AUTHORIZE_URL")
        .unwrap_or_else(|_| "https://discord.com/oauth2/authorize".to_string());
    (StatusCode::TEMPORARY_REDIRECT, Redirect::temporary(&url))
}
