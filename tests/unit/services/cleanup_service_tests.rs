use chrono::{Utc, Duration};
use socs_backend::repositories::user_repository;
use socs_backend::services::cleanup_service;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_cleanup_expired_soft_deletes(pool: PgPool) {
    let email = format!("cleanup_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Cleanup User", &email, "hash").await.unwrap();

    let project_id_1 = Uuid::new_v4();
    let project_id_2 = Uuid::new_v4();

    // Expired soft delete (> 30 days)
    sqlx::query(
        "INSERT INTO projects (id, slug, title, description, status, created_by, deleted_at) VALUES ($1, $2, $3, $4, 'pending'::content_status, $5, $6)"
    )
    .bind(project_id_1)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Expired Project")
    .bind("Desc")
    .bind(user.id)
    .bind(Utc::now() - Duration::days(40))
    .execute(&pool)
    .await
    .unwrap();

    // Recent soft delete (< 30 days)
    sqlx::query(
        "INSERT INTO projects (id, slug, title, description, status, created_by, deleted_at) VALUES ($1, $2, $3, $4, 'pending'::content_status, $5, $6)"
    )
    .bind(project_id_2)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Recent Project")
    .bind("Desc")
    .bind(user.id)
    .bind(Utc::now() - Duration::days(10))
    .execute(&pool)
    .await
    .unwrap();

    let stats = cleanup_service::cleanup_expired_soft_deletes(&pool).await.unwrap();
    
    assert!(stats.projects_purged >= 1);
    
    // Check project 1 was permanently deleted
    let count1: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects WHERE id = $1")
        .bind(project_id_1)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count1.0, 0);

    // Check project 2 is still there (soft deleted)
    let count2: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects WHERE id = $1")
        .bind(project_id_2)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count2.0, 1);
}
