use axum::{body::Body, http::Request};
use ezgif_server::{app_state::AppState, router::build_router};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;

#[tokio::test]
async fn health_route_returns_ok() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}
