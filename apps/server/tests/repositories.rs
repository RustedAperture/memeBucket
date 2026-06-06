use ezgif_server::repositories::{
    images::ImageRepository, pools::PoolRepository, users::UserRepository,
};
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn user_category_and_media_link_are_scoped_to_owner() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());

    let alice = users
        .upsert_by_discord_key("alice-key", Some("Alice"), None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob-key", Some("Bob"), None)
        .await
        .unwrap();

    let cats = pools.create(alice.id, "cats").await.unwrap();
    images
        .create(alice.id, cats.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let alice_pools = pools.list_for_user(alice.id).await.unwrap();
    let bob_pools = pools.list_for_user(bob.id).await.unwrap();

    assert_eq!(alice_pools.len(), 1);
    assert_eq!(alice_pools[0].name, "cats");
    assert!(bob_pools.is_empty());
}

#[tokio::test]
async fn category_names_are_unique_per_user_case_insensitive() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();

    pools.create(user.id, "cats").await.unwrap();
    let duplicate = pools.create(user.id, "Cats").await;

    assert!(matches!(duplicate, Err(sqlx::Error::RowNotFound)));
}

#[tokio::test]
async fn media_link_rejects_category_owned_by_different_user() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());

    let alice = users
        .upsert_by_discord_key("alice-scope-key", Some("Alice"), None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob-scope-key", Some("Bob"), None)
        .await
        .unwrap();

    let alice_pool = pools.create(alice.id, "cats").await.unwrap();
    let result = images
        .create(bob.id, alice_pool.id, "https://example.com/not-allowed.gif")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn media_link_rejects_orphan_category_id() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());

    let user = users
        .upsert_by_discord_key("orphan-key", Some("Owner"), None)
        .await
        .unwrap();

    let result = images
        .create(
            user.id,
            uuid::Uuid::new_v4(),
            "https://example.com/missing.gif",
        )
        .await;

    assert!(result.is_err());
}
