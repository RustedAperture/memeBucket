use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{Request, StatusCode},
    routing::get,
};
use ezgif_server::{app_state::AppState, router::build_router_for_tests};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower::ServiceExt;

#[tokio::test]
async fn gif_search_reuses_cached_klipy_response_for_same_query() {
    let upstream_calls = Arc::new(AtomicUsize::new(0));
    let klipy_base_url = spawn_klipy_mock(upstream_calls.clone()).await;

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let state = AppState::for_tests(pool)
        .with_klipy_api_key(Some("test-key".to_string()))
        .with_klipy_api_base_url(klipy_base_url);
    let app = build_router_for_tests(state);

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=Cat&per_page=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second_response = app
        .oneshot(
            Request::builder()
                .uri("/api/gifs/search?q=%20cat%20&per_page=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_body = first_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let second_body = second_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    let first_json: serde_json::Value = serde_json::from_slice(&first_body).unwrap();

    assert_eq!(first_json["data"]["per_page"], 50);
    assert_eq!(first_body, second_body);
    assert_eq!(upstream_calls.load(Ordering::SeqCst), 1);
}

async fn spawn_klipy_mock(upstream_calls: Arc<AtomicUsize>) -> String {
    let app = Router::new()
        .route("/{*path}", get(klipy_response))
        .with_state(upstream_calls);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{address}")
}

async fn klipy_response(
    State(upstream_calls): State<Arc<AtomicUsize>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let call_count = upstream_calls.fetch_add(1, Ordering::SeqCst) + 1;
    let per_page = params
        .get("per_page")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(24);

    Json(json!({
        "data": {
            "per_page": per_page,
            "data": [
                {
                    "id": "cached-result",
                    "url": format!("https://example.com/{call_count}.gif")
                }
            ]
        }
    }))
}
