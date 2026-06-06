use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::sessions::{create_session, session_cookie},
    domain::user_key::DiscordUserKey,
    repositories::users::UserRepository,
};

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: Option<String>,
    global_name: Option<String>,
    avatar: Option<String>,
}

pub async fn start_discord_oauth() -> impl IntoResponse {
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("DISCORD_OAUTH_REDIRECT_URL").unwrap_or_default();

    let url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify%20applications.commands&integration_type=1",
        urlencoding(&client_id),
        urlencoding(&redirect_uri),
    );

    (StatusCode::TEMPORARY_REDIRECT, Redirect::temporary(&url))
}

pub async fn handle_discord_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    if query.code.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match complete_oauth_flow(&state, &query.code).await {
        Ok(session_id) => {
            let cookie = session_cookie(&session_id.to_string());
            (
                StatusCode::TEMPORARY_REDIRECT,
                [(axum::http::header::SET_COOKIE, cookie)],
                Redirect::temporary("/"),
            )
                .into_response()
        }
        Err(err) => {
            tracing::warn!("OAuth flow failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn complete_oauth_flow(state: &AppState, code: &str) -> anyhow::Result<Uuid> {
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = std::env::var("DISCORD_OAUTH_REDIRECT_URL").unwrap_or_default();

    // Exchange code for access token
    let http = reqwest::Client::new();
    let token_response: DiscordTokenResponse = http
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // Fetch Discord user profile
    let discord_user: DiscordUser = http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_response.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // Derive user key and upsert
    let secret = std::env::var("APP_USER_KEY_SECRET").unwrap_or_default();
    let user_key = DiscordUserKey::derive(secret.as_bytes(), &discord_user.id);
    let display_name = discord_user
        .global_name
        .or(discord_user.username)
        .unwrap_or_default();

    let avatar_url = discord_user.avatar.map(|hash| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            discord_user.id, hash
        )
    });

    let users = UserRepository::new(state.pool.clone());
    let stored_user = users
        .upsert_by_discord_key(
            user_key.as_hex(),
            Some(&display_name),
            avatar_url.as_deref(),
        )
        .await?;

    // Create session
    let session_id = create_session(&state.pool, stored_user.id).await?;

    Ok(session_id)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
