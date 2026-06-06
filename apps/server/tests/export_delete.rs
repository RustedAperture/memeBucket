use ezgif_server::repositories::{
    images::ImageRepository, pools::PoolRepository, send_history::SendHistoryRepository,
    users::UserRepository,
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
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let service = AccountService::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", Some("Alice"), None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    images
        .create(user.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    let other_user = users
        .upsert_by_discord_key("other-user-key", Some("Bob"), None)
        .await
        .unwrap();
    let other_pool = pools.create(other_user.id, "dogs").await.unwrap();
    images
        .create(other_user.id, other_pool.id, "https://example.com/dog.gif")
        .await
        .unwrap();

    let export = service.export_user_data(user.id).await.unwrap();

    assert_eq!(export.pools.len(), 1);
    assert_eq!(export.pools[0].name, "cats");
    assert_eq!(export.pools[0].images[0].url, "https://example.com/cat.gif");
    assert!(export.pools.iter().all(|pool| pool.name != "dogs"));
    assert!(export.pools.iter().all(|pool| {
        pool.images
            .iter()
            .all(|image| image.url != "https://example.com/dog.gif")
    }));
}

#[tokio::test]
async fn delete_account_removes_owned_data() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let pools = PoolRepository::new(pool.clone());
    let images = ImageRepository::new(pool.clone());
    let send_history = SendHistoryRepository::new(pool.clone());
    let service = AccountService::new(pool.clone());
    let user = users
        .upsert_by_discord_key("user-key", None, None)
        .await
        .unwrap();
    let saved_pool = pools.create(user.id, "cats").await.unwrap();
    images
        .create(user.id, saved_pool.id, "https://example.com/cat.gif")
        .await
        .unwrap();
    let image = images
        .list_for_pool(user.id, saved_pool.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    send_history
        .record(user.id, &saved_pool, &image, "public")
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
    let other_pool = pools.create(other_user.id, "dogs").await.unwrap();
    images
        .create(other_user.id, other_pool.id, "https://example.com/dog.gif")
        .await
        .unwrap();
    let other_image = images
        .list_for_pool(other_user.id, other_pool.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    send_history
        .record(other_user.id, &other_pool, &other_image, "private")
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
    let deleted_pool_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pools WHERE owner_user_id = ?")
            .bind(user.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    let deleted_image_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE owner_user_id = ?")
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
    assert_eq!(deleted_pool_count, 0);
    assert_eq!(deleted_image_count, 0);
    assert_eq!(deleted_send_history_count, 0);
    assert!(pools.list_for_user(user.id).await.unwrap().is_empty());
    assert!(
        images
            .list_for_pool(user.id, saved_pool.id)
            .await
            .unwrap()
            .is_empty()
    );

    assert_eq!(surviving_user_count, 1);
    assert_eq!(surviving_session_count, 1);
    assert_eq!(surviving_send_history_count, 1);
    assert_eq!(pools.list_for_user(other_user.id).await.unwrap().len(), 1);
    assert_eq!(
        images
            .list_for_pool(other_user.id, other_pool.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
