use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ezgif_server::{app_state::AppState, router::build_router};
use sqlx::SqlitePool;
use tower::ServiceExt;

#[tokio::test]
async fn category_api_requires_session() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/categories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oauth_start_route_is_public() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/discord/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}
