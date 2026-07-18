use memebucket_server::repositories::{
    BucketRepo, ImageRepo, SendHistoryRepo, UserRepo, buckets::BucketRepository,
    images::ImageRepository, send_history::SendHistoryRepository, users::UserRepository,
};
use memebucket_server::services::{
    images::validate_http_url,
    random::{RandomError, RandomService, RandomVisibility},
};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn random_lookup_matches_bucket_case_insensitively() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history),
    );
    let user = users
        .upsert_by_provider("discord", "user-key", None, None)
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

    assert_eq!(selected.bucket_name, "Cats");
    assert_eq!(selected.url, "https://example.com/cat.gif");

    let row = sqlx::query(
        "SELECT bucket_name, url, response_visibility FROM send_history WHERE owner_user_id = ?",
    )
    .bind(user.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("bucket_name"), "Cats");
    assert_eq!(row.get::<String, _>("url"), "https://example.com/cat.gif");
    assert_eq!(row.get::<String, _>("response_visibility"), "private");
}

#[tokio::test]
async fn random_lookup_combines_images_from_multiple_pools() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history),
    );
    let user = users
        .upsert_by_provider("discord", "multi-pool-user-key", None, None)
        .await
        .unwrap();
    pools.create(user.id, "Cats").await.unwrap();
    let dogs = pools.create(user.id, "Dogs").await.unwrap();
    images
        .create(user.id, dogs.id, "https://example.com/dog.gif")
        .await
        .unwrap();

    let selected = service
        .select_random_from_buckets(user.id, &["cats", "dogs"], RandomVisibility::Public)
        .await
        .unwrap();

    assert_eq!(selected.bucket_name, "Dogs");
    assert_eq!(selected.url, "https://example.com/dog.gif");

    let row = sqlx::query(
        "SELECT bucket_name, url, response_visibility FROM send_history WHERE owner_user_id = ?",
    )
    .bind(user.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("bucket_name"), "Dogs");
    assert_eq!(row.get::<String, _>("url"), "https://example.com/dog.gif");
    assert_eq!(row.get::<String, _>("response_visibility"), "public");
}

#[tokio::test]
async fn random_excludes_zero_weight_images() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history),
    );
    let user = users
        .upsert_by_provider("discord", "zero-weight-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/skip.gif",
            None,
            false,
            0,
            &[],
        )
        .await
        .unwrap();
    images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/use.gif",
            None,
            false,
            1,
            &[],
        )
        .await
        .unwrap();

    for _ in 0..10 {
        let selected = service
            .select_random(user.id, "cats", RandomVisibility::Public)
            .await
            .unwrap();
        assert_eq!(selected.url, "https://example.com/use.gif");
    }
}

#[tokio::test]
async fn random_avoids_recent_image_when_alternative_exists() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history.clone()),
    );
    let user = users
        .upsert_by_provider("discord", "recent-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    let recent = images
        .create(user.id, saved_pool.id, "https://example.com/recent.gif")
        .await
        .unwrap();
    let fresh = images
        .create(user.id, saved_pool.id, "https://example.com/fresh.gif")
        .await
        .unwrap();
    history
        .record(user.id, saved_pool.id, recent.id, "public")
        .await
        .unwrap();

    let selected = service
        .select_random(user.id, "cats", RandomVisibility::Public)
        .await
        .unwrap();

    assert_eq!(selected.url, fresh.url);
}

#[tokio::test]
async fn random_returns_only_image_from_single_image_pool() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history.clone()),
    );
    let user = users
        .upsert_by_provider("discord", "single-image-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    let only = images
        .create(user.id, saved_pool.id, "https://example.com/only.gif")
        .await
        .unwrap();
    history
        .record(user.id, saved_pool.id, only.id, "public")
        .await
        .unwrap();

    let selected = service
        .select_random(user.id, "cats", RandomVisibility::Public)
        .await
        .unwrap();

    assert_eq!(selected.url, only.url);
}

#[tokio::test]
async fn all_zero_weight_images_return_random_enabled_error() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history),
    );
    let user = users
        .upsert_by_provider("discord", "all-zero-random-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/zero-a.gif",
            None,
            false,
            0,
            &[],
        )
        .await
        .unwrap();
    images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/zero-b.gif",
            None,
            true,
            0,
            &[],
        )
        .await
        .unwrap();

    let err = service
        .select_random(user.id, "cats", RandomVisibility::Private)
        .await
        .unwrap_err();

    assert!(matches!(err, RandomError::NoRandomEnabledImages));
    assert_eq!(
        err.user_message(),
        "That bucket has no random-enabled images yet."
    );
    assert!(err.is_private());
}

#[tokio::test]
async fn empty_bucket_returns_private_safe_error() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(Arc::new(pools.clone()), Arc::new(images), Arc::new(history));
    let user = users
        .upsert_by_provider("discord", "user-key", None, None)
        .await
        .unwrap();
    pools.create(user.id, "empty").await.unwrap();

    let err = service
        .select_random(user.id, "empty", RandomVisibility::Public)
        .await
        .unwrap_err();

    assert_eq!(err.user_message(), "That bucket has no saved images yet.");
    assert!(err.is_private());
}

#[tokio::test]
async fn send_history_record_rejects_cross_owner_inputs() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());

    let alice = users
        .upsert_by_provider("discord", "alice-history-key", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_provider("discord", "bob-history-key", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(alice.id, "Cats").await.unwrap();
    let image = images
        .create(alice.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let err = history
        .record(bob.id, saved_pool.id, image.id, "public")
        .await
        .unwrap_err();

    assert!(matches!(err, sqlx::Error::RowNotFound));
}

#[tokio::test]
async fn recent_image_ids_for_buckets_prefers_latest_insert_when_sent_at_ties() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let user = users
        .upsert_by_provider("discord", "recent-order-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "Cats").await.unwrap();
    let first = images
        .create(user.id, saved_pool.id, "https://example.com/first.gif")
        .await
        .unwrap();
    let second = images
        .create(user.id, saved_pool.id, "https://example.com/second.gif")
        .await
        .unwrap();

    history
        .record(user.id, saved_pool.id, first.id, "public")
        .await
        .unwrap();
    history
        .record(user.id, saved_pool.id, second.id, "public")
        .await
        .unwrap();

    let recent_ids = history
        .recent_image_ids_for_buckets(user.id, &[saved_pool.id], 1)
        .await
        .unwrap();

    assert_eq!(recent_ids, vec![second.id]);
}

#[tokio::test]
async fn subscribed_pool_random_selection_records_requesting_user_history() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(
        Arc::new(pools.clone()),
        Arc::new(images.clone()),
        Arc::new(history),
    );
    let owner = users
        .upsert_by_provider("discord", "owner-subscribed-history", None, None)
        .await
        .unwrap();
    let subscriber = users
        .upsert_by_provider("discord", "subscriber-subscribed-history", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(owner.id, "Cats").await.unwrap();
    images
        .create(owner.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    pools
        .subscribe_user_to_bucket(subscriber.id, saved_pool.id)
        .await
        .unwrap();

    let selected = service
        .select_random(subscriber.id, "cats", RandomVisibility::Public)
        .await
        .unwrap();

    assert_eq!(selected.bucket_name, "Cats");
    assert_eq!(selected.url, "https://example.com/cat.gif");

    let row = sqlx::query(
        "SELECT bucket_name, url, response_visibility FROM send_history WHERE owner_user_id = ?",
    )
    .bind(subscriber.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("bucket_name"), "Cats");
    assert_eq!(row.get::<String, _>("url"), "https://example.com/cat.gif");
    assert_eq!(row.get::<String, _>("response_visibility"), "public");
}

#[tokio::test]
async fn storage_failures_are_reported_as_storage_errors() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = BucketRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let history = SendHistoryRepository::new(pool.clone());
    let service = RandomService::new(Arc::new(pools.clone()), Arc::new(images), Arc::new(history));
    let user = users
        .upsert_by_provider("discord", "storage-user-key", None, None)
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
