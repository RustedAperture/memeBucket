use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::SqlitePool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

pub fn hash_csrf_token(session_secret: &str, token: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes())
        .expect("HMAC accepts secrets of any length");
    mac.update(token.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_csrf_token(session_secret: &str, token: &str, expected_hash: &str) -> bool {
    let Ok(expected_bytes) = hex::decode(expected_hash) else {
        return false;
    };

    let mut mac = HmacSha256::new_from_slice(session_secret.as_bytes())
        .expect("HMAC accepts secrets of any length");
    mac.update(token.as_bytes());
    mac.verify_slice(&expected_bytes).is_ok()
}

pub fn session_cookie(value: &str) -> String {
    format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        value
    )
}

pub fn expired_session_cookie() -> String {
    "session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0".to_string()
}

pub async fn create_session(pool: &SqlitePool, user_id: Uuid) -> Result<Uuid, sqlx::Error> {
    let session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token_hash, expires_at) VALUES (?, ?, '', datetime('now', '+24 hours'))",
    )
    .bind(session_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(session_id)
}

pub async fn lookup_session(pool: &SqlitePool, session_id: &str) -> Option<AuthenticatedUser> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT user_id FROM sessions WHERE id = ? AND expires_at > datetime('now')",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    Uuid::parse_str(&row.0)
        .ok()
        .map(|user_id| AuthenticatedUser { user_id })
}

pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn read_session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("session="))
                .map(str::to_string)
        })
}
