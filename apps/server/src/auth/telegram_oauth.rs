use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::{
    app_state::AppState,
    auth::sessions::{
        create_session, csrf_cookie, lookup_session, read_session_cookie, session_cookie,
    },
};

type HmacSha256 = Hmac<Sha256>;

#[derive(serde::Deserialize, Debug)]
pub struct TelegramCallbackQuery {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: Option<i64>,
    pub hash: Option<String>,
}

pub async fn handle_telegram_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TelegramCallbackQuery>,
) -> impl IntoResponse {
    let bot_token = state.telegram_bot_token();
    if bot_token.is_empty() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let hash = match &query.hash {
        Some(h) => h.clone(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let auth_date = query.auth_date.unwrap_or(0);

    // Replay protection: reject payloads older than 1 hour
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if now - auth_date > 3600 {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Build check string: alphabetically sorted "key=value" pairs joined by \n
    // Include all fields except hash
    let mut fields: BTreeMap<&str, String> = BTreeMap::new();
    fields.insert("id", query.id.clone());
    fields.insert("auth_date", auth_date.to_string());
    if let Some(ref v) = query.first_name {
        fields.insert("first_name", v.clone());
    }
    if let Some(ref v) = query.last_name {
        fields.insert("last_name", v.clone());
    }
    if let Some(ref v) = query.username {
        fields.insert("username", v.clone());
    }
    if let Some(ref v) = query.photo_url {
        fields.insert("photo_url", v.clone());
    }

    let check_string = fields
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    // Secret key = SHA256(bot_token)
    let secret_key = Sha256::digest(bot_token.as_bytes());
    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC accepts any key length");
    mac.update(check_string.as_bytes());
    let computed = hex::encode(mac.finalize().into_bytes());

    if computed != hash {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Auth verified. Determine display name.
    let display_name = query
        .first_name
        .as_deref()
        .or(query.username.as_deref())
        .map(str::to_string);
    let avatar_url = query.photo_url.clone();

    // Check for existing session: link mode (active session) vs login mode (no session)
    let active_user = if let Some(session_id) = read_session_cookie(&headers) {
        lookup_session(&state.pool, &session_id).await
    } else {
        None
    };

    let user = if let Some(active) = active_user {
        // Link mode: add Telegram identity to existing user
        let already_linked = state
            .user_repo
            .get_identities(active.user_id)
            .await
            .unwrap_or_default()
            .iter()
            .any(|i| i.provider == "telegram");

        if !already_linked
            && let Err(e) = state
                .user_repo
                .link_identity(
                    active.user_id,
                    "telegram",
                    &query.id,
                    display_name.as_deref(),
                    avatar_url.as_deref(),
                )
                .await
        {
            tracing::warn!(
                "Failed to link Telegram identity for user {}: {e}",
                active.user_id
            );
        }

        // Redirect back to settings
        return Redirect::to("/settings#connected-accounts").into_response();
    } else {
        // Login mode: look up or create user
        match state
            .user_repo
            .upsert_by_provider(
                "telegram",
                &query.id,
                display_name.as_deref(),
                avatar_url.as_deref(),
            )
            .await
        {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("DB error upserting Telegram user: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    };

    let user_id = user.id;
    match create_session(&state.pool, user_id, state.session_secret()).await {
        Ok((session_id, csrf_token)) => {
            let mut response_headers = HeaderMap::new();
            if let Ok(c) = session_cookie(&session_id.to_string()).parse() {
                response_headers.append(axum::http::header::SET_COOKIE, c);
            }
            if let Ok(c) = csrf_cookie(&csrf_token).parse() {
                response_headers.append(axum::http::header::SET_COOKIE, c);
            }
            (
                StatusCode::TEMPORARY_REDIRECT,
                response_headers,
                Redirect::temporary("/"),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Session creation failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
