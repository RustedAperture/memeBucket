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
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let redirect_uri =
        std::env::var("DISCORD_OAUTH_REDIRECT_URL").unwrap_or_default();

    let url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify",
        urlencoding(&client_id),
        urlencoding(&redirect_uri),
    );

    (StatusCode::TEMPORARY_REDIRECT, Redirect::temporary(&url))
}

pub async fn handle_discord_oauth_callback(
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    if query.code.trim().is_empty() || query.state.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // TODO: exchange code for token, upsert user, create session
    Redirect::temporary("/").into_response()
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
