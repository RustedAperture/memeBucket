use std::{ffi::OsString, net::SocketAddr};

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
    repositories::{
        images::ImageRepository, pools::PoolRepository, send_history::SendHistoryRepository,
        users::UserRepository,
    },
    router::build_router_for_tests,
};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tokio::{net::TcpListener, sync::Mutex};
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

async fn spawn_image_server() -> String {
    async fn image() -> Response {
        (
            [(header::CONTENT_TYPE, "image/gif")],
            "GIF89a-test-image-bytes",
        )
            .into_response()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().route("/cat.gif", get(image));

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{address}/cat.gif")
}

async fn read_json(response: axum::response::Response) -> serde_json::Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn search_images_filters_by_text_tags_favorite_and_random_state() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-search-owner", None, None)
        .await
        .unwrap();
    let cats = pools.create(owner.id, "Cats").await.unwrap();
    let dogs = pools.create(owner.id, "Dogs").await.unwrap();
    let happy = images
        .create_with_metadata(
            owner.id,
            cats.id,
            "https://example.com/happy-cat.gif",
            Some("Happy Cat"),
            true,
            4,
            &["cat".to_string(), "reaction".to_string()],
        )
        .await
        .unwrap();
    images
        .create_with_metadata(
            owner.id,
            dogs.id,
            "https://example.com/sad-dog.gif",
            Some("Sad Dog"),
            false,
            0,
            &["dog".to_string(), "reaction".to_string()],
        )
        .await
        .unwrap();
    history
        .record(owner.id, &cats, &happy, "private")
        .await
        .unwrap();

    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let mut request = Request::builder()
        .uri(format!(
            "/api/images/search?q=happy&tags=cat&favorite=true&randomEnabled=true&poolId={}",
            cats.id
        ))
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let results = read_json(response).await;

    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["poolId"], cats.id.to_string());
    assert_eq!(results[0]["poolName"], "Cats");
    assert_eq!(results[0]["image"]["id"], happy.id.to_string());
    assert_eq!(results[0]["image"]["title"], "Happy Cat");
    assert_eq!(results[0]["image"]["favorite"], true);
    assert_eq!(results[0]["image"]["randomWeight"], 4);
    assert_eq!(results[0]["image"]["sendCount"], 1);
    assert_eq!(
        results[0]["image"]["tags"],
        serde_json::json!(["cat", "reaction"])
    );
}

#[tokio::test]
async fn search_images_returns_only_accessible_images() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let alice = users
        .upsert_by_discord_key("image-search-alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("image-search-bob", None, None)
        .await
        .unwrap();
    let alice_pool = pools.create(alice.id, "Alice Cats").await.unwrap();
    let bob_pool = pools.create(bob.id, "Bob Cats").await.unwrap();
    let visible = images
        .create_with_metadata(
            alice.id,
            alice_pool.id,
            "https://example.com/visible-cat.gif",
            Some("Visible Cat"),
            false,
            1,
            &["cat".to_string()],
        )
        .await
        .unwrap();
    images
        .create_with_metadata(
            bob.id,
            bob_pool.id,
            "https://example.com/secret-cat.gif",
            Some("Secret Cat"),
            false,
            1,
            &["cat".to_string()],
        )
        .await
        .unwrap();

    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let mut request = Request::builder()
        .uri("/api/images/search?q=cat")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: alice.id });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let results = read_json(response).await;

    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["poolName"], "Alice Cats");
    assert_eq!(results[0]["image"]["id"], visible.id.to_string());
}

#[tokio::test]
async fn create_image_accepts_metadata_and_returns_normalized_fields() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("EZGIF_ALLOW_LOCAL_IPS_IN_TESTS", "1");

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-api-owner", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let image_url = spawn_image_server().await;

    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"url":"{image_url}","title":"Shocked Cat","tags":["cat","reaction","cat"]}}"#
        )))
        .unwrap();
    request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = read_json(response).await;

    assert_eq!(created["url"], image_url);
    assert_eq!(created["title"], "Shocked Cat");
    assert_eq!(created["favorite"], false);
    assert_eq!(created["randomWeight"], 1);
    assert_eq!(created["sendCount"], 0);
    assert_eq!(created["tags"], serde_json::json!(["cat", "reaction"]));
}

#[tokio::test]
async fn patch_image_updates_metadata_and_list_shows_new_values() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("EZGIF_ALLOW_LOCAL_IPS_IN_TESTS", "1");

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-api-owner-update", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let image_url = spawn_image_server().await;

    let mut create_request = Request::builder()
        .method("POST")
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"url":"{image_url}","title":"Shocked Cat","tags":["cat","reaction","cat"]}}"#
        )))
        .unwrap();
    create_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = read_json(create_response).await;
    let image_id = created["id"].as_str().unwrap();

    let mut patch_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"title":"Better Cat","notes":"credit: example","favorite":true,"randomWeight":4,"tags":["favorite","surprised"]}"#,
        ))
        .unwrap();
    patch_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let patch_response = app.clone().oneshot(patch_request).await.unwrap();
    assert_eq!(patch_response.status(), StatusCode::OK);

    let mut list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let images = read_json(list_response).await;
    let image = &images.as_array().unwrap()[0];

    assert_eq!(image["title"], "Better Cat");
    assert_eq!(image["notes"], "credit: example");
    assert_eq!(image["favorite"], true);
    assert_eq!(image["randomWeight"], 4);
    assert_eq!(image["sendCount"], 0);
    assert_eq!(image["tags"], serde_json::json!(["favorite", "surprised"]));

    let mut favorite_only_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"favorite":false}"#))
        .unwrap();
    favorite_only_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let favorite_only_response = app.clone().oneshot(favorite_only_request).await.unwrap();
    assert_eq!(favorite_only_response.status(), StatusCode::OK);

    let mut preserved_list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    preserved_list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let preserved_list_response = app.clone().oneshot(preserved_list_request).await.unwrap();
    assert_eq!(preserved_list_response.status(), StatusCode::OK);
    let preserved_images = read_json(preserved_list_response).await;
    let preserved_image = &preserved_images.as_array().unwrap()[0];

    assert_eq!(preserved_image["title"], "Better Cat");
    assert_eq!(preserved_image["notes"], "credit: example");
    assert_eq!(preserved_image["favorite"], false);
    assert_eq!(preserved_image["randomWeight"], 4);
    assert_eq!(
        preserved_image["tags"],
        serde_json::json!(["favorite", "surprised"])
    );

    let mut clear_tags_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"tags":[]}"#))
        .unwrap();
    clear_tags_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let clear_tags_response = app.clone().oneshot(clear_tags_request).await.unwrap();
    assert_eq!(clear_tags_response.status(), StatusCode::OK);

    let mut cleared_tags_list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    cleared_tags_list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let cleared_tags_list_response = app.oneshot(cleared_tags_list_request).await.unwrap();
    assert_eq!(cleared_tags_list_response.status(), StatusCode::OK);
    let cleared_tags_images = read_json(cleared_tags_list_response).await;
    let cleared_tags_image = &cleared_tags_images.as_array().unwrap()[0];

    assert_eq!(cleared_tags_image["title"], "Better Cat");
    assert_eq!(cleared_tags_image["notes"], "credit: example");
    assert_eq!(cleared_tags_image["randomWeight"], 4);
    assert_eq!(cleared_tags_image["tags"], serde_json::json!([]));
}

#[tokio::test]
async fn subscriber_cannot_patch_owner_image_metadata() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("EZGIF_ALLOW_LOCAL_IPS_IN_TESTS", "1");

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-api-owner-subscriber", None, None)
        .await
        .unwrap();
    let subscriber = users
        .upsert_by_discord_key("image-api-subscriber", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    pools
        .subscribe_user_to_pool(subscriber.id, saved_pool.id)
        .await
        .unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let image_url = spawn_image_server().await;

    let mut create_request = Request::builder()
        .method("POST")
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"url":"{image_url}","title":"Shocked Cat","tags":["cat","reaction","cat"]}}"#
        )))
        .unwrap();
    create_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = read_json(create_response).await;
    let image_id = created["id"].as_str().unwrap();

    let mut patch_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"title":"Better Cat","notes":"credit: example","favorite":true,"randomWeight":4,"tags":["favorite","surprised"]}"#,
        ))
        .unwrap();
    patch_request.extensions_mut().insert(AuthenticatedUser {
        user_id: subscriber.id,
    });

    let patch_response = app.clone().oneshot(patch_request).await.unwrap();
    assert_eq!(patch_response.status(), StatusCode::NOT_FOUND);

    let mut invalid_patch_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"randomWeight":999}"#))
        .unwrap();
    invalid_patch_request
        .extensions_mut()
        .insert(AuthenticatedUser {
            user_id: subscriber.id,
        });

    let invalid_patch_response = app.clone().oneshot(invalid_patch_request).await.unwrap();
    assert_eq!(invalid_patch_response.status(), StatusCode::NOT_FOUND);

    let mut list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let images = read_json(list_response).await;
    let image = &images.as_array().unwrap()[0];

    assert_eq!(image["title"], "Shocked Cat");
    assert_eq!(image["favorite"], false);
    assert_eq!(image["randomWeight"], 1);
    assert_eq!(image["tags"], serde_json::json!(["cat", "reaction"]));
}

#[tokio::test]
async fn patch_notes_distinguishes_omitted_from_null() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("EZGIF_ALLOW_LOCAL_IPS_IN_TESTS", "1");

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-api-owner-notes", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);
    let image_url = spawn_image_server().await;

    let mut create_request = Request::builder()
        .method("POST")
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"url":"{image_url}","title":"Shocked Cat","tags":["cat"]}}"#
        )))
        .unwrap();
    create_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = read_json(create_response).await;
    let image_id = created["id"].as_str().unwrap();

    let mut set_notes_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"notes":"credit: example"}"#))
        .unwrap();
    set_notes_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let set_notes_response = app.clone().oneshot(set_notes_request).await.unwrap();
    assert_eq!(set_notes_response.status(), StatusCode::OK);

    let mut omit_notes_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"title":"Still Cat","favorite":true,"randomWeight":2,"tags":["cat","favorite"]}"#,
        ))
        .unwrap();
    omit_notes_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let omit_notes_response = app.clone().oneshot(omit_notes_request).await.unwrap();
    assert_eq!(omit_notes_response.status(), StatusCode::OK);

    let mut list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let images = read_json(list_response).await;
    let image = &images.as_array().unwrap()[0];
    assert_eq!(image["notes"], "credit: example");

    let mut clear_notes_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/pools/{}/images/{image_id}", saved_pool.id))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"notes":null}"#))
        .unwrap();
    clear_notes_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let clear_notes_response = app.clone().oneshot(clear_notes_request).await.unwrap();
    assert_eq!(clear_notes_response.status(), StatusCode::OK);

    let mut final_list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    final_list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let final_list_response = app.oneshot(final_list_request).await.unwrap();
    assert_eq!(final_list_response.status(), StatusCode::OK);
    let final_images = read_json(final_list_response).await;
    let final_image = &final_images.as_array().unwrap()[0];
    assert!(final_image["notes"].is_null());
}

#[tokio::test]
async fn list_images_returns_nonzero_send_count_for_requester() {
    let _local_ip_guard = LOCAL_IP_TEST_LOCK.lock().await;
    let _allow_local_ips = EnvVarGuard::set("EZGIF_ALLOW_LOCAL_IPS_IN_TESTS", "1");

    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let send_history = SendHistoryRepository::new(pool.clone());
    let owner = users
        .upsert_by_discord_key("image-api-owner-send-count", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "cats").await.unwrap();
    let image_url = spawn_image_server().await;
    let created_image = images
        .create_with_metadata(
            owner.id,
            saved_pool.id,
            &image_url,
            Some("Counter Cat"),
            false,
            1,
            &["counted".to_string()],
        )
        .await
        .unwrap();

    send_history
        .record(owner.id, &saved_pool, &created_image, "public")
        .await
        .unwrap();
    send_history
        .record(owner.id, &saved_pool, &created_image, "private")
        .await
        .unwrap();

    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    let mut list_request = Request::builder()
        .uri(format!("/api/pools/{}/images", saved_pool.id))
        .body(Body::empty())
        .unwrap();
    list_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: owner.id });

    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let images = read_json(list_response).await;
    let image = &images.as_array().unwrap()[0];

    assert_eq!(image["title"], "Counter Cat");
    assert_eq!(image["sendCount"], 2);
}
