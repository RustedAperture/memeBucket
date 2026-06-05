use ezgif_server::repositories::{
    categories::CategoryRepository, media_links::MediaLinkRepository, users::UserRepository,
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
    let categories = CategoryRepository::new(pool.clone());
    let links = MediaLinkRepository::new(pool.clone());

    let alice = users
        .upsert_by_discord_key("alice-key", Some("Alice"), None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob-key", Some("Bob"), None)
        .await
        .unwrap();

    let cats = categories.create(alice.id, "cats").await.unwrap();
    links
        .create(alice.id, cats.id, "https://example.com/cat.gif")
        .await
        .unwrap();

    let alice_categories = categories.list_for_user(alice.id).await.unwrap();
    let bob_categories = categories.list_for_user(bob.id).await.unwrap();

    assert_eq!(alice_categories.len(), 1);
    assert_eq!(alice_categories[0].name, "cats");
    assert!(bob_categories.is_empty());
}

#[tokio::test]
async fn category_names_are_unique_per_user_case_insensitive() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let categories = CategoryRepository::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();

    categories.create(user.id, "cats").await.unwrap();
    let duplicate = categories.create(user.id, "Cats").await;

    assert!(matches!(duplicate, Err(sqlx::Error::RowNotFound)));
}

#[tokio::test]
async fn media_link_rejects_category_owned_by_different_user() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let categories = CategoryRepository::new(pool.clone());
    let links = MediaLinkRepository::new(pool.clone());

    let alice = users
        .upsert_by_discord_key("alice-scope-key", Some("Alice"), None)
        .await
        .unwrap();
    let bob = users
        .upsert_by_discord_key("bob-scope-key", Some("Bob"), None)
        .await
        .unwrap();

    let alice_category = categories.create(alice.id, "cats").await.unwrap();
    let result = links
        .create(
            bob.id,
            alice_category.id,
            "https://example.com/not-allowed.gif",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn media_link_rejects_orphan_category_id() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let links = MediaLinkRepository::new(pool.clone());

    let user = users
        .upsert_by_discord_key("orphan-key", Some("Owner"), None)
        .await
        .unwrap();

    let result = links
        .create(
            user.id,
            uuid::Uuid::new_v4(),
            "https://example.com/missing.gif",
        )
        .await;

    assert!(result.is_err());
}
