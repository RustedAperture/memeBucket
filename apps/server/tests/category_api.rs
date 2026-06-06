use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use ezgif_server::{
    app_state::AppState,
    auth::sessions::AuthenticatedUser,
    repositories::{pools::PoolRepository, users::UserRepository},
    router::build_router_for_tests,
};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower::ServiceExt;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn delete_category_requires_owner_scope() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let alice = users
        .upsert_by_discord_key("alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(alice.id, "cats").await.unwrap();

    let deleted = pools.delete_for_user(bob.id, saved_pool.id).await.unwrap();

    assert!(!deleted);
    assert_eq!(pools.list_for_user(alice.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn category_routes_support_owner_scoped_crud() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut create_request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  Cats  "}"#))
        .unwrap();
    create_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = create_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let created: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    assert_eq!(created["name"], "Cats");
    let pool_id = created["id"].as_str().unwrap().to_string();

    let mut list_request = Request::builder()
        .uri("/api/pools")
        .body(Body::empty())
        .unwrap();
    list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let pools: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(pools.as_array().unwrap().len(), 1);
    assert_eq!(pools[0]["name"], "Cats");

    let mut delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/pools/{pool_id}"))
        .body(Body::empty())
        .unwrap();
    delete_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let delete_response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);
    let delete_body = delete_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let deleted: serde_json::Value = serde_json::from_slice(&delete_body).unwrap();
    assert_eq!(deleted, serde_json::json!({ "deleted": true }));
}

#[tokio::test]
async fn create_category_rejects_blank_name() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"   "}"#))
        .unwrap();
    request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"bad request: pool name is required");
}

#[tokio::test]
async fn create_category_rejects_duplicate_name_for_same_owner() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut first_request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Cats"}"#))
        .unwrap();
    first_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let first_response = app.clone().oneshot(first_request).await.unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);

    let mut duplicate_request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  cAtS  "}"#))
        .unwrap();
    duplicate_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let duplicate_response = app.clone().oneshot(duplicate_request).await.unwrap();
    assert_eq!(duplicate_response.status(), StatusCode::BAD_REQUEST);
    let body = duplicate_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(&body[..], b"bad request: pool already exists");
}

#[tokio::test]
async fn create_category_allows_same_name_for_different_owners() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let alice = users
        .upsert_by_discord_key("alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut alice_request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Cats"}"#))
        .unwrap();
    alice_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: alice.id });

    let alice_response = app.clone().oneshot(alice_request).await.unwrap();
    assert_eq!(alice_response.status(), StatusCode::OK);

    let mut bob_request = Request::builder()
        .method("POST")
        .uri("/api/pools")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  cats  "}"#))
        .unwrap();
    bob_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: bob.id });

    let bob_response = app.clone().oneshot(bob_request).await.unwrap();
    assert_eq!(bob_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_image_stores_resolved_metadata_image_url() {
    async fn page(address: String) -> Response {
        (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            format!(
                r#"<html><head><meta name="twitter:image" content="http://{address}/image.gif"></head></html>"#
            ),
        )
            .into_response()
    }

    async fn image() -> Response {
        ([(header::CONTENT_TYPE, "image/gif")], "gif").into_response()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let media_address = listener.local_addr().unwrap().to_string();
    let media_app = Router::new()
        .route(
            "/",
            get({
                let media_address = media_address.clone();
                move || page(media_address)
            }),
        )
        .route("/image.gif", get(image));

    tokio::spawn(async move {
        axum::serve(listener, media_app).await.unwrap();
    });

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let page_url = format!("http://{media_address}/");
    let expected_image_url = format!("http://{media_address}/image.gif");

    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"url":"{page_url}"}}"#)))
        .unwrap();
    request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(created["url"], expected_image_url);
}
