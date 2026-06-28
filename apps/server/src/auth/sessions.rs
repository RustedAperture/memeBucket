use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::SqlitePool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub role: String, // "user" or "admin"
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

fn is_cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
}

pub fn session_cookie(value: &str) -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax;{} Max-Age=2592000",
        value, secure
    )
}

pub fn csrf_cookie(value: &str) -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!(
        "csrf_token={}; Path=/; SameSite=Lax;{} Max-Age=2592000",
        value, secure
    )
}

pub fn expired_session_cookie() -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!(
        "session=; Path=/; HttpOnly; SameSite=Lax;{} Max-Age=0",
        secure
    )
}

pub fn expired_csrf_cookie() -> String {
    let secure = if is_cookie_secure() { " Secure;" } else { "" };
    format!("csrf_token=; Path=/; SameSite=Lax;{} Max-Age=0", secure)
}

pub async fn create_session(
    pool: &SqlitePool,
    user_id: Uuid,
    session_secret: &str,
) -> Result<(Uuid, String), sqlx::Error> {
    let session_id = Uuid::new_v4();
    let csrf_token = Uuid::new_v4().to_string();
    let csrf_token_hash = hash_csrf_token(session_secret, &csrf_token);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token_hash, expires_at) VALUES (?, ?, ?, datetime('now', '+30 days'))",
    )
    .bind(session_id.to_string())
    .bind(user_id.to_string())
    .bind(csrf_token_hash)
    .execute(pool)
    .await?;
    Ok((session_id, csrf_token))
}

pub struct SessionInfo {
    pub user_id: Uuid,
    pub csrf_token_hash: String,
}

pub async fn lookup_session_info(pool: &SqlitePool, session_id: &str) -> Option<SessionInfo> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT user_id, csrf_token_hash FROM sessions WHERE id = ? AND expires_at > datetime('now')",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    Uuid::parse_str(&row.0).ok().map(|user_id| SessionInfo {
        user_id,
        csrf_token_hash: row.1,
    })
}

pub async fn lookup_session(pool: &SqlitePool, session_id: &str) -> Option<AuthenticatedUser> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"SELECT s.user_id, u.role
           FROM sessions s
           JOIN users u ON u.id = s.user_id
           WHERE s.id = ? AND s.expires_at > datetime('now')"#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    Uuid::parse_str(&row.0)
        .ok()
        .map(|user_id| AuthenticatedUser {
            user_id,
            role: row.1,
        })
}

pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn read_session_cookie(headers: &HeaderMap) -> Option<String> {
    read_cookie(headers, "session")
}

pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    for cookie_header in headers.get_all("cookie").iter() {
        if let Ok(cookies_str) = cookie_header.to_str() {
            for cookie in cookies_str.split(';').map(str::trim) {
                if let Some(value) = cookie.strip_prefix(&format!("{name}=")) {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}
