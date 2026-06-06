use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ezgif_server::{app_state::AppState, router::build_router_for_tests};
use sqlx::SqlitePool;
use tower::ServiceExt;

#[tokio::test]
async fn category_api_requires_session() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/api/pools")
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
    let app = build_router_for_tests(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/auth/discord/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}
