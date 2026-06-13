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

#[tokio::test]
async fn image_create_persists_title_weight_favorite_and_tags() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("metadata-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();

    let created = images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/cat.gif",
            Some("Shocked Cat"),
            false,
            3,
            &[
                "Cat".to_string(),
                " reaction ".to_string(),
                "cat".to_string(),
            ],
        )
        .await
        .unwrap();

    assert_eq!(created.title.as_deref(), Some("Shocked Cat"));
    assert!(!created.favorite);
    assert_eq!(created.random_weight, 3);
    assert_eq!(
        created.tags,
        vec!["Cat".to_string(), "reaction".to_string()]
    );
}

#[tokio::test]
async fn image_metadata_update_requires_owner() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let alice = users
        .upsert_by_discord_key("metadata-alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("metadata-bob", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(alice.id, "cats").await.unwrap();
    let image = images
        .create(alice.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let updated = images
        .update_metadata(
            bob.id,
            saved_pool.id,
            image.id,
            Some("Nope"),
            Some("notes"),
            true,
            5,
            &["stolen".to_string()],
        )
        .await
        .unwrap();

    assert!(!updated);
}

#[tokio::test]
async fn image_move_rejects_target_pool_owned_by_different_user() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let alice = users
        .upsert_by_discord_key("move-alice", None, None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("move-bob", None, None)
        .await
        .unwrap();
    let alice_pool = pools.create(alice.id, "cats").await.unwrap();
    let bob_pool = pools.create(bob.id, "dogs").await.unwrap();
    let image = images
        .create(alice.id, alice_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let moved = images
        .move_to_pool(alice.id, alice_pool.id, image.id, bob_pool.id)
        .await
        .unwrap();

    assert!(!moved);

    let alice_images = images.list_for_pool(alice.id, alice_pool.id).await.unwrap();
    assert_eq!(alice_images.len(), 1);
    assert_eq!(alice_images[0].id, image.id);
    assert_eq!(alice_images[0].pool_id, alice_pool.id);

    let bob_images = images.list_for_pool(bob.id, bob_pool.id).await.unwrap();
    assert!(bob_images.is_empty());
}

#[tokio::test]
async fn image_list_preserves_tag_order_from_metadata() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("tag-order-user", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();

    images
        .create_with_metadata(
            user.id,
            saved_pool.id,
            "https://example.com/cat.gif",
            Some("Ordered Cat"),
            false,
            1,
            &[
                "third".to_string(),
                "first".to_string(),
                "second".to_string(),
            ],
        )
        .await
        .unwrap();

    let listed = images.list_for_pool(user.id, saved_pool.id).await.unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].tags,
        vec![
            "third".to_string(),
            "first".to_string(),
            "second".to_string(),
        ]
    );
}
