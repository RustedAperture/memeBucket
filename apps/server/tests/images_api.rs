use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use memebucket_server::{
    app_state::AppState,
    auth::sessions::AuthenticatedUser,
    repositories::{
        BucketRepo, ImageRepo, UserRepo, buckets::BucketRepository, images::ImageRepository,
        users::UserRepository,
    },
    router::build_router_for_tests,
};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_bulk_delete_and_move_images() {
    let pool = test_pool().await;
    let users = UserRepository::new(pool.clone());
    let buckets = BucketRepository::new(pool.clone());
    let images_repo = ImageRepository::new(pool.clone());

    let user = users
        .upsert_by_provider("discord", "owner", None, None)
        .await
        .unwrap();

    let bucket_a = buckets.create(user.id, "Bucket A").await.unwrap();
    let bucket_b = buckets.create(user.id, "Bucket B").await.unwrap();

    // Create 3 images in bucket A
    let img1 = images_repo
        .create(user.id, bucket_a.id, "https://example.com/1.png")
        .await
        .unwrap();
    let img2 = images_repo
        .create(user.id, bucket_a.id, "https://example.com/2.png")
        .await
        .unwrap();
    let img3 = images_repo
        .create(user.id, bucket_a.id, "https://example.com/3.png")
        .await
        .unwrap();

    let state = AppState::for_tests(pool);
    let app = build_router_for_tests(state);

    // 1. Test Bulk Move: move img1 and img2 to bucket B
    let payload_move = serde_json::json!({
        "imageIds": [img1.id.to_string(), img2.id.to_string()],
        "newBucketId": bucket_b.id.to_string()
    });
    let mut move_request = Request::builder()
        .method("POST")
        .uri(format!("/api/buckets/{}/images/bulk/move", bucket_a.id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload_move).unwrap()))
        .unwrap();
    move_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let move_response = app.clone().oneshot(move_request).await.unwrap();
    assert_eq!(move_response.status(), StatusCode::OK);

    let move_body = move_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let move_json: serde_json::Value = serde_json::from_slice(&move_body).unwrap();
    assert_eq!(move_json["moved"], 2);

    // Verify images in bucket A and bucket B
    let images_a = images_repo
        .list_for_bucket(user.id, bucket_a.id)
        .await
        .unwrap();
    let images_b = images_repo
        .list_for_bucket(user.id, bucket_b.id)
        .await
        .unwrap();
    assert_eq!(images_a.len(), 1); // Only img3 remains
    assert_eq!(images_a[0].id, img3.id);
    assert_eq!(images_b.len(), 2); // img1 and img2 moved here

    // 2. Test Bulk Delete: delete img1 and img2 from bucket B
    let payload_delete = serde_json::json!({
        "imageIds": [img1.id.to_string(), img2.id.to_string()]
    });
    let mut delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/buckets/{}/images/bulk", bucket_b.id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload_delete).unwrap()))
        .unwrap();
    delete_request
        .extensions_mut()
        .insert(AuthenticatedUser { user_id: user.id });

    let delete_response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);

    let delete_body = delete_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let delete_json: serde_json::Value = serde_json::from_slice(&delete_body).unwrap();
    assert_eq!(delete_json["deleted"], 2);

    // Verify bucket B is now empty
    let images_b_after = images_repo
        .list_for_bucket(user.id, bucket_b.id)
        .await
        .unwrap();
    assert_eq!(images_b_after.len(), 0);
}
