use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use http_body_util::BodyExt;
use memebucket_server::{
    app_state::AppState,
    auth::sessions::AuthenticatedUser,
    repositories::{
        BucketRepo, ImageRepo, SendHistoryRepo, UserRepo, buckets::BucketRepository,
        images::ImageRepository, send_history::SendHistoryRepository, users::UserRepository,
    },
    router::build_router_for_tests,
};
use sqlx::SqlitePool;
use std::ffi::OsString;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceExt;

static LOCAL_IP_TEST_LOCK: Mutex<()> = Mutex::const_new(());

struct EnvVarGuard {
    name: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(name);
        unsafe {
            std::env::set_var(name, value);
        }
        Self { name, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var(self.name, value);
            },
            None => unsafe {
                std::env::remove_var(self.name);
            },
        }
    }
}

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn delete_bucket_requires_owner_scope() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let alice = users
        .upsert_by_provider("discord", "alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_provider("discord", "bob", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(alice.id, "cats").await.unwrap();

    let deleted = pools.delete_for_user(bob.id, saved_pool.id).await.unwrap();

    assert!(!deleted);
    assert_eq!(pools.list_for_user(alice.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn bucket_routes_support_owner_scoped_crud() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_provider("discord", "owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut create_request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  Cats  "}"#))
        .unwrap();
    create_request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

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
    let bucket_id = created["id"].as_str().unwrap().to_string();

    let mut list_request = Request::builder()
        .uri("/api/buckets")
        .body(Body::empty())
        .unwrap();
    list_request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

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
        .uri(format!("/api/buckets/{bucket_id}"))
        .body(Body::empty())
        .unwrap();
    delete_request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

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
async fn create_bucket_rejects_blank_name() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_provider("discord", "owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"   "}"#))
        .unwrap();
    request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"bad request: bucket name is required");
}

#[tokio::test]
async fn create_bucket_rejects_duplicate_name_for_same_owner() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let user = users
        .upsert_by_provider("discord", "owner", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut first_request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Cats"}"#))
        .unwrap();
    first_request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

    let first_response = app.clone().oneshot(first_request).await.unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);

    let mut duplicate_request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  cAtS  "}"#))
        .unwrap();
    duplicate_request
        .extensions_mut()
        .insert(AuthenticatedUser {
            user_id: user.id,
            role: "user".to_string(),
        });

    let duplicate_response = app.clone().oneshot(duplicate_request).await.unwrap();
    assert_eq!(duplicate_response.status(), StatusCode::BAD_REQUEST);
    let body = duplicate_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(&body[..], b"bad request: bucket already exists");
}

#[tokio::test]
async fn create_bucket_allows_same_name_for_different_owners() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let alice = users
        .upsert_by_provider("discord", "alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_provider("discord", "bob", None, None)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut alice_request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Cats"}"#))
        .unwrap();
    alice_request.extensions_mut().insert(AuthenticatedUser {
        user_id: alice.id,
        role: "user".to_string(),
    });

    let alice_response = app.clone().oneshot(alice_request).await.unwrap();
    assert_eq!(alice_response.status(), StatusCode::OK);

    let mut bob_request = Request::builder()
        .method("POST")
        .uri("/api/buckets")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"  cats  "}"#))
        .unwrap();
    bob_request.extensions_mut().insert(AuthenticatedUser {
        user_id: bob.id,
        role: "user".to_string(),
    });

    let bob_response = app.clone().oneshot(bob_request).await.unwrap();
    assert_eq!(bob_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_image_stores_resolved_metadata_image_url() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("MEMEBUCKET_ALLOW_LOCAL_IPS_IN_TESTS", "1");

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
    let pools = BucketRepository::new(pool.clone());
    let user = users
        .upsert_by_provider("discord", "owner", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let page_url = format!("http://{media_address}/");
    let expected_image_url = format!("http://{media_address}/image.gif");

    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/buckets/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"url":"{page_url}"}}"#)))
        .unwrap();
    request.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        role: "user".to_string(),
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(created["url"], expected_image_url);
}

#[tokio::test]
async fn whitelist_enabled_blocks_existing_subscriber_image_access() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let owner = users
        .upsert_by_provider("discord", "owner-whitelist", None, None)
        .await
        .unwrap();
    let subscriber = users
        .upsert_by_provider("discord", "subscriber-whitelist", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    images
        .create(owner.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    pools
        .subscribe_user_to_bucket(subscriber.id, saved_pool.id)
        .await
        .unwrap();

    assert_eq!(
        images
            .list_for_bucket(subscriber.id, saved_pool.id)
            .await
            .unwrap()
            .len(),
        1
    );

    pools
        .set_whitelist_enabled(saved_pool.id, owner.id, true)
        .await
        .unwrap();

    assert!(
        images
            .list_for_bucket(subscriber.id, saved_pool.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn shared_pool_preview_uses_viewer_send_count_and_anonymous_gets_zero() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let send_history = SendHistoryRepository::new(pool.clone());
    let owner = users
        .upsert_by_provider("discord", "owner-shared-preview", None, None)
        .await
        .unwrap();
    let subscriber = users
        .upsert_by_provider("discord", "subscriber-shared-preview", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    let image = images
        .create_with_metadata(
            owner.id,
            saved_pool.id,
            "https://example.com/cat.gif",
            Some("Preview Cat"),
            false,
            1,
            &["shared".to_string()],
        )
        .await
        .unwrap();
    pools
        .subscribe_user_to_bucket(subscriber.id, saved_pool.id)
        .await
        .unwrap();
    pools
        .set_share_token(saved_pool.id, owner.id, Some("share1"))
        .await
        .unwrap();

    send_history
        .record(owner.id, saved_pool.id, image.id, "public")
        .await
        .unwrap();
    send_history
        .record(owner.id, saved_pool.id, image.id, "private")
        .await
        .unwrap();
    send_history
        .record(subscriber.id, saved_pool.id, image.id, "public")
        .await
        .unwrap();

    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut subscriber_request = Request::builder()
        .uri("/api/share/share1")
        .body(Body::empty())
        .unwrap();
    subscriber_request
        .extensions_mut()
        .insert(AuthenticatedUser {
            user_id: subscriber.id,
            role: "user".to_string(),
        });

    let subscriber_response = app.clone().oneshot(subscriber_request).await.unwrap();
    assert_eq!(subscriber_response.status(), StatusCode::OK);
    let subscriber_body = subscriber_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let subscriber_preview: serde_json::Value = serde_json::from_slice(&subscriber_body).unwrap();
    assert_eq!(subscriber_preview["images"][0]["title"], "Preview Cat");
    assert_eq!(subscriber_preview["images"][0]["sendCount"], 1);

    let anonymous_response = app
        .oneshot(
            Request::builder()
                .uri("/api/share/share1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous_response.status(), StatusCode::OK);
    let anonymous_body = anonymous_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let anonymous_preview: serde_json::Value = serde_json::from_slice(&anonymous_body).unwrap();
    assert_eq!(anonymous_preview["images"][0]["sendCount"], 0);
}
