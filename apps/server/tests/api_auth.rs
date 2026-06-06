use axum::{
    body::Body,
    http::{Request, StatusCode, header},
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

#[tokio::test]
async fn oauth_start_sets_state_cookie_and_redirect_param() {
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
    let location = response
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    let redirect_url = url::Url::parse(location).unwrap();
    let state_param = redirect_url
        .query_pairs()
        .find_map(|(key, value)| (key == "state").then_some(value.into_owned()))
        .unwrap();
    let set_cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|value| value.starts_with("oauth_state="))
        .unwrap();

    assert!(state_param.len() >= 32);
    assert!(set_cookie.contains(&format!("oauth_state={state_param}")));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
}

#[tokio::test]
async fn oauth_callback_rejects_missing_state_before_token_exchange() {
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
                .uri("/auth/discord/callback?code=test-code")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
