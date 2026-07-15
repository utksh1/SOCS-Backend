use socs_backend::repositories::{resource_repository, user_repository};
use socs_backend::models::resource::{Resource, ResourceCategory, ContentStatus};
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_test_resource(pool: &PgPool, title: &str, user_id: Uuid) -> Resource {
    sqlx::query_as::<_, Resource>(
        r#"
        INSERT INTO resources (title, description, category, url, tags, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(title)
    .bind("Test Description")
    .bind(ResourceCategory::Tool)
    .bind("https://example.com/tool")
    .bind(vec!["rust".to_string()])
    .bind(ContentStatus::Pending)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("Failed to insert test resource")
}

#[sqlx::test]
async fn test_find_by_id(pool: PgPool) {
    let email = format!("res_user_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Resource User", &email, "hash").await.unwrap();
    let resource = insert_test_resource(&pool, "Test Resource ID", user.id).await;

    let found = resource_repository::find_by_id(&pool, resource.id).await.unwrap().unwrap();
    assert_eq!(found.id, resource.id);
    assert_eq!(found.title, "Test Resource ID");
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("res_user_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Resource User Del", &email, "hash").await.unwrap();
    let resource = insert_test_resource(&pool, "Test Resource Del", user.id).await;

    let deleted = resource_repository::soft_delete(&pool, resource.id).await.unwrap();
    assert!(deleted);

    let found = resource_repository::find_by_id(&pool, resource.id).await.unwrap();
    assert!(found.is_none());

    let restored = resource_repository::restore(&pool, resource.id).await.unwrap();
    assert!(restored);

    let found_restored = resource_repository::find_by_id(&pool, resource.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("res_user_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Resource User Perm", &email, "hash").await.unwrap();
    let resource = insert_test_resource(&pool, "Test Resource Perm", user.id).await;

    resource_repository::soft_delete(&pool, resource.id).await.unwrap();
    let deleted = resource_repository::permanent_delete(&pool, resource.id).await.unwrap();
    assert!(deleted);

    let found = resource_repository::find_by_id(&pool, resource.id).await.unwrap();
    assert!(found.is_none());
}
