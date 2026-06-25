use memebucket_server::{
    repositories::{buckets::BucketRepository, images::ImageRepository, users::UserRepository},
    services::account::AccountService,
};
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_export_import_and_delete_account() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let buckets = BucketRepository::new(pool.clone());
    let images_repo = ImageRepository::new(pool.clone());
    let account_service = AccountService::new(pool.clone());

    // 1. Create user, bucket, and image with full metadata
    let user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();

    let bucket = buckets.create(user.id, "Memes").await.unwrap();

    let _image = images_repo
        .create_with_metadata(
            user.id,
            bucket.id,
            "https://example.com/meme.jpg",
            Some("Funny Cat"),
            true,
            5,
            &["cat".to_string(), "funny".to_string()],
        )
        .await
        .unwrap();

    // 2. Export user data
    let exported = account_service.export_user_data(user.id).await.unwrap();
    assert_eq!(exported.buckets.len(), 1);
    assert_eq!(exported.buckets[0].name, "Memes");
    assert_eq!(exported.buckets[0].images.len(), 1);

    // 3. Delete account (this deletes all buckets and images due to cascade delete)
    account_service.delete_account(user.id).await.unwrap();

    // Verify user, bucket, and images are gone
    assert!(users.get_by_id(user.id).await.unwrap().is_none());
    assert_eq!(buckets.list_for_user(user.id).await.unwrap().len(), 0);

    // 4. Re-create user and import the exported data
    let new_user = users
        .upsert_by_discord_key("owner", None, None)
        .await
        .unwrap();

    let (buckets_created, images_created) = account_service
        .import_user_data(new_user.id, exported)
        .await
        .unwrap();

    assert_eq!(buckets_created, 1);
    assert_eq!(images_created, 1);

    // 5. Verify imported data is 100% correct
    let imported_buckets = buckets.list_for_user(new_user.id).await.unwrap();
    assert_eq!(imported_buckets.len(), 1);
    assert_eq!(imported_buckets[0].name, "Memes");

    let imported_images = images_repo
        .list_for_bucket(new_user.id, imported_buckets[0].id)
        .await
        .unwrap();
    assert_eq!(imported_images.len(), 1);
    assert_eq!(imported_images[0].url, "https://example.com/meme.jpg");
    assert_eq!(imported_images[0].title.as_deref(), Some("Funny Cat"));
    assert!(imported_images[0].favorite);
    assert_eq!(imported_images[0].random_weight, 5);
    assert_eq!(
        imported_images[0].tags,
        vec!["cat".to_string(), "funny".to_string()]
    );
}
