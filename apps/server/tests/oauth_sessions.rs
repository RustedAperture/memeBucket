use ezgif_server::{
    auth::sessions::{create_session, hash_csrf_token, lookup_session_info, verify_csrf_token},
    repositories::users::UserRepository,
};
use sqlx::SqlitePool;

#[test]
fn csrf_hash_verifies_original_token() {
    let hash = hash_csrf_token("session-secret", "csrf-token");

    assert!(verify_csrf_token("session-secret", "csrf-token", &hash));
    assert!(!verify_csrf_token("session-secret", "wrong-token", &hash));
}

#[tokio::test]
async fn create_session_uses_explicit_session_secret_for_csrf_hash() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("csrf-secret-user", None, None)
        .await
        .unwrap();

    let (session_id, csrf_token) = create_session(&pool, user.id, "explicit-secret")
        .await
        .unwrap();

    let session = lookup_session_info(&pool, &session_id.to_string())
        .await
        .unwrap();
    assert!(verify_csrf_token(
        "explicit-secret",
        &csrf_token,
        &session.csrf_token_hash
    ));
    assert!(!verify_csrf_token(
        "",
        &csrf_token,
        &session.csrf_token_hash
    ));
}
