use ezgif_server::repositories::{
    categories::CategoryRepository, media_links::MediaLinkRepository,
    send_history::SendHistoryRepository, users::UserRepository,
};
use ezgif_server::services::account::AccountService;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn export_contains_user_owned_categories_and_links() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let categories = CategoryRepository::new(pool.clone());
    let links = MediaLinkRepository::new(pool.clone());
    let service = AccountService::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", Some("Alice"), None)
        .await
        .unwrap();
    let category = categories.create(user.id, "cats").await.unwrap();
    links
        .create(user.id, category.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    let other_user = users
        .upsert_by_discord_key("other-user-key", Some("Bob"), None)
        .await
        .unwrap();
    let other_category = categories.create(other_user.id, "dogs").await.unwrap();
    links
        .create(
            other_user.id,
            other_category.id,
            "https://example.com/dog.gif",
        )
        .await
        .unwrap();

    let export = service.export_user_data(user.id).await.unwrap();

    assert_eq!(export.categories.len(), 1);
    assert_eq!(export.categories[0].name, "cats");
    assert_eq!(
        export.categories[0].links[0].url,
        "https://example.com/cat.gif"
    );
    assert!(
        export
            .categories
            .iter()
            .all(|category| category.name != "dogs")
    );
    assert!(export.categories.iter().all(|category| {
        category
            .links
            .iter()
            .all(|link| link.url != "https://example.com/dog.gif")
    }));
}

#[tokio::test]
async fn delete_account_removes_owned_data() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let categories = CategoryRepository::new(pool.clone());
    let links = MediaLinkRepository::new(pool.clone());
    let send_history = SendHistoryRepository::new(pool.clone());
    let service = AccountService::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();
    let category = categories.create(user.id, "cats").await.unwrap();
    links
        .create(user.id, category.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    let media_link = links
        .list_for_category(user.id, category.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    send_history
        .record(user.id, &category, &media_link, "public")
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id.to_string())
    .bind("csrf-hash")
    .bind("2099-01-01T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();

    let other_user = users
        .upsert_by_discord_key("other-user-key", Some("Bob"), None)
        .await
        .unwrap();
    let other_category = categories.create(other_user.id, "dogs").await.unwrap();
    links
        .create(
            other_user.id,
            other_category.id,
            "https://example.com/dog.gif",
        )
        .await
        .unwrap();
    let other_media_link = links
        .list_for_category(other_user.id, other_category.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    send_history
        .record(other_user.id, &other_category, &other_media_link, "private")
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(other_user.id.to_string())
    .bind("other-csrf-hash")
    .bind("2099-01-01T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();

    service.delete_account(user.id).await.unwrap();

    let deleted_user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(user.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    let deleted_session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
            .bind(user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let deleted_category_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE owner_user_id = ?")
            .bind(user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let deleted_media_link_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_links WHERE owner_user_id = ?")
            .bind(user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let deleted_send_history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM send_history WHERE owner_user_id = ?")
            .bind(user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

    let surviving_user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(other_user.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    let surviving_session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
            .bind(other_user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let surviving_send_history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM send_history WHERE owner_user_id = ?")
            .bind(other_user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(deleted_user_count, 0);
    assert_eq!(deleted_session_count, 0);
    assert_eq!(deleted_category_count, 0);
    assert_eq!(deleted_media_link_count, 0);
    assert_eq!(deleted_send_history_count, 0);
    assert!(categories.list_for_user(user.id).await.unwrap().is_empty());
    assert!(
        links
            .list_for_category(user.id, category.id)
            .await
            .unwrap()
            .is_empty()
    );

    assert_eq!(surviving_user_count, 1);
    assert_eq!(surviving_session_count, 1);
    assert_eq!(surviving_send_history_count, 1);
    assert_eq!(
        categories.list_for_user(other_user.id).await.unwrap().len(),
        1
    );
    assert_eq!(
        links
            .list_for_category(other_user.id, other_category.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
