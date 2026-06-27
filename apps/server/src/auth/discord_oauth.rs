use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
};
use rand::Rng;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::sessions::{create_session, read_cookie, session_cookie},
    domain::user_key::DiscordUserKey,
};

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: Option<String>,
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
    let state = random_oauth_state();

    let url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify%20applications.commands&integration_type=1&state={}",
        urlencoding(&client_id),
        urlencoding(&redirect_uri),
        urlencoding(&state),
    );

    let mut headers = HeaderMap::new();
    if let Ok(cookie) = oauth_state_cookie(&state).parse() {
        headers.append(axum::http::header::SET_COOKIE, cookie);
    }

    (
        StatusCode::TEMPORARY_REDIRECT,
        headers,
        Redirect::temporary(&url),
    )
}

pub async fn handle_discord_oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    if query.code.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if !valid_oauth_state(&headers, query.state.as_deref()) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match complete_oauth_flow(&state, &query.code).await {
        Ok((session_id, csrf_token)) => {
            let cookie = session_cookie(&session_id.to_string());
            let csrf_cookie_str = crate::auth::sessions::csrf_cookie(&csrf_token);
            let mut headers = axum::http::HeaderMap::new();
            if let Ok(c) = cookie.parse() {
                headers.append(axum::http::header::SET_COOKIE, c);
            }
            if let Ok(c) = csrf_cookie_str.parse() {
                headers.append(axum::http::header::SET_COOKIE, c);
            }
            if let Ok(c) = expired_oauth_state_cookie().parse() {
                headers.append(axum::http::header::SET_COOKIE, c);
            }
            (
                StatusCode::TEMPORARY_REDIRECT,
                headers,
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

async fn complete_oauth_flow(state: &AppState, code: &str) -> anyhow::Result<(Uuid, String)> {
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
    let secret = std::env::var("APP_USER_KEY_SECRET")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("APP_USER_KEY_SECRET is required"))?;
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

    let users = state.user_repo.clone();
    let stored_user = users
        .upsert_by_discord_key(
            user_key.as_hex(),
            Some(&display_name),
            avatar_url.as_deref(),
        )
        .await?;

    // Create session
    let (session_id, csrf_token) =
        create_session(&state.pool, stored_user.id, state.session_secret()).await?;

    Ok((session_id, csrf_token))
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn random_oauth_state() -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn is_cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
}

fn oauth_state_cookie(value: &str) -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!(
        "oauth_state={value}; Path=/auth/discord/callback; HttpOnly; SameSite=Lax;{} Max-Age=600",
        secure
    )
}

fn expired_oauth_state_cookie() -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!(
        "oauth_state=; Path=/auth/discord/callback; HttpOnly; SameSite=Lax;{} Max-Age=0",
        secure
    )
}

fn valid_oauth_state(headers: &HeaderMap, state: Option<&str>) -> bool {
    let Some(state) = state.filter(|value| !value.is_empty()) else {
        return false;
    };
    read_cookie(headers, "oauth_state").is_some_and(|expected| expected == state)
}
