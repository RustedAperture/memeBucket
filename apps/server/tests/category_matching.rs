use ezgif_server::repositories::{
    images::ImageRepository, pools::PoolRepository, send_history::SendHistoryRepository,
    users::UserRepository,
};
use ezgif_server::services::{
    images::validate_http_url,
    random::{RandomError, RandomService, RandomVisibility},
};
use sqlx::{Row, SqlitePool};

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn random_lookup_matches_category_case_insensitively() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(pools.clone(), images.clone(), history);
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "Cats").await.unwrap();
    images
        .create(user.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let selected = service
        .select_random(user.id, "cats", RandomVisibility::Private)
        .await
        .unwrap();

    assert_eq!(selected.pool_name, "Cats");
    assert_eq!(selected.url, "https://example.com/cat.gif");

    let row = sqlx::query(
        "SELECT pool_name, url, response_visibility FROM send_history WHERE owner_user_id = ?",
    )
    .bind(user.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("pool_name"), "Cats");
    assert_eq!(row.get::<String, _>("url"), "https://example.com/cat.gif");
    assert_eq!(row.get::<String, _>("response_visibility"), "private");
}

#[tokio::test]
async fn random_lookup_combines_images_from_multiple_pools() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(pools.clone(), images.clone(), history);
    let user = users
        .upsert_by_discord_key("multi-pool-user-key", None, None)
        .await
        .unwrap();
    pools.create(user.id, "Cats").await.unwrap();
    let dogs = pools.create(user.id, "Dogs").await.unwrap();
    images
        .create(user.id, dogs.id, "https://example.com/dog.gif")
        .await
        .unwrap();

    let selected = service
        .select_random_from_pools(user.id, &["cats", "dogs"], RandomVisibility::Public)
        .await
        .unwrap();

    assert_eq!(selected.pool_name, "Dogs");
    assert_eq!(selected.url, "https://example.com/dog.gif");

    let row = sqlx::query(
        "SELECT pool_name, url, response_visibility FROM send_history WHERE owner_user_id = ?",
    )
    .bind(user.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("pool_name"), "Dogs");
    assert_eq!(row.get::<String, _>("url"), "https://example.com/dog.gif");
    assert_eq!(row.get::<String, _>("response_visibility"), "public");
}

#[tokio::test]
async fn empty_category_returns_private_safe_error() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(pools.clone(), images, history);
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();
    pools.create(user.id, "empty").await.unwrap();

    let err = service
        .select_random(user.id, "empty", RandomVisibility::Public)
        .await
        .unwrap_err();

    assert_eq!(err.user_message(), "That pool has no saved images yet.");
    assert!(err.is_private());
}

#[tokio::test]
async fn send_history_record_rejects_cross_owner_inputs() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());

    let alice = users
        .upsert_by_discord_key("alice-history-key", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob-history-key", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(alice.id, "Cats").await.unwrap();
    let image = images
        .create(alice.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let err = history
        .record(bob.id, &saved_pool, &image, "public")
        .await
        .unwrap_err();

    assert!(matches!(err, sqlx::Error::RowNotFound));
}

#[tokio::test]
async fn storage_failures_are_reported_as_storage_errors() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(pools.clone(), images, history);
    let user = users
        .upsert_by_discord_key("storage-user-key", None, None)
        .await
        .unwrap();
    pools.create(user.id, "cats").await.unwrap();

    pool.close().await;

    let err = service
        .select_random(user.id, "cats", RandomVisibility::Private)
        .await
        .unwrap_err();

    assert!(matches!(err, RandomError::Storage(_)));
    assert_ne!(err.user_message(), "I could not find that pool.");
    assert_ne!(err.user_message(), "That pool has no saved images yet.");
    assert!(err.is_private());
}

#[test]
fn validate_http_url_accepts_http_and_https() {
    assert!(validate_http_url("http://example.com/cat.gif"));
    assert!(validate_http_url("https://example.com/cat.gif"));
}

#[test]
fn validate_http_url_rejects_non_http_and_invalid_inputs() {
    assert!(!validate_http_url("ftp://example.com/cat.gif"));
    assert!(!validate_http_url("not a url"));
    assert!(!validate_http_url("https://"));
    assert!(!validate_http_url(" https://example.com/cat.gif "));
}
