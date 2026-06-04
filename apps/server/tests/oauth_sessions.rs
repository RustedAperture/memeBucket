use random_media_bot_server::auth::sessions::{hash_csrf_token, verify_csrf_token};

#[test]
fn csrf_hash_verifies_original_token() {
    let hash = hash_csrf_token("session-secret", "csrf-token");

    assert!(verify_csrf_token("session-secret", "csrf-token", &hash));
    assert!(!verify_csrf_token("session-secret", "wrong-token", &hash));
}
