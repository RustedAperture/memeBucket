use axum::{
    body::Body,
    body::Bytes,
    http::{Request, StatusCode, header::HeaderName},
};
use ed25519_dalek::Signer;
use ezgif_server::{app_state::AppState, router::build_router};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

#[tokio::test]
async fn unsigned_interaction_is_rejected() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/discord/interactions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"type":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn signed_ping_payload_returns_pong_with_configured_public_key() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let body = Bytes::from_static(br#"{"type":1}"#);
    let timestamp = "1717171717";
    let signature = signing_key
        .sign(&[timestamp.as_bytes(), body.as_ref()].concat())
        .to_bytes();
    let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

    let state = AppState::for_tests(pool).with_discord_public_key(public_key_hex);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/discord/interactions")
                .header("content-type", "application/json")
                .header(
                    HeaderName::from_static("x-signature-ed25519"),
                    hex::encode(signature),
                )
                .header(HeaderName::from_static("x-signature-timestamp"), timestamp)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], br#"{"type":1}"#);
}
