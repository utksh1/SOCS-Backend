use socs_backend::repositories::user_repository;
use socs_backend::services::project_service;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_soft_delete_project_cascades(pool: PgPool) {
    let email = format!("proj_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User", &email, "hash").await.unwrap();

    let project_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO projects (id, slug, title, description, status, created_by) VALUES ($1, $2, $3, $4, 'pending'::content_status, $5)"
    )
    .bind(project_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Project")
    .bind("Desc")
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    // Insert project feature
    sqlx::query(
        "INSERT INTO project_features (project_id, title, description) VALUES ($1, $2, $3)"
    )
    .bind(project_id)
    .bind("Feature 1")
    .bind("Feature Desc")
    .execute(&pool)
    .await
    .unwrap();

    // Soft delete
    let result = project_service::soft_delete_project(&pool, project_id).await;
    assert!(result.is_ok());

    // Project should be soft deleted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects WHERE id = $1 AND deleted_at IS NULL")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    // Feature should be soft deleted
    let count_feature: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM project_features WHERE project_id = $1 AND deleted_at IS NULL")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_feature.0, 0);
}

#[sqlx::test]
async fn test_restore_project_cascades(pool: PgPool) {
    let email = format!("proj_rest_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User", &email, "hash").await.unwrap();

    let project_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO projects (id, slug, title, description, status, created_by) VALUES ($1, $2, $3, $4, 'pending'::content_status, $5)"
    )
    .bind(project_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Project")
    .bind("Desc")
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO project_features (project_id, title, description) VALUES ($1, $2, $3)"
    )
    .bind(project_id)
    .bind("Feature 1")
    .bind("Feature Desc")
    .execute(&pool)
    .await
    .unwrap();

    project_service::soft_delete_project(&pool, project_id).await.unwrap();

    // Restore
    let result = project_service::restore_project(&pool, project_id).await;
    assert!(result.is_ok());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects WHERE id = $1 AND deleted_at IS NULL")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    let count_feature: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM project_features WHERE project_id = $1 AND deleted_at IS NULL")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_feature.0, 1);
}

#[sqlx::test]
async fn test_permanent_delete_project(pool: PgPool) {
    let email = format!("proj_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User", &email, "hash").await.unwrap();

    let project_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO projects (id, slug, title, description, status, created_by) VALUES ($1, $2, $3, $4, 'pending'::content_status, $5)"
    )
    .bind(project_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Project")
    .bind("Desc")
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    // Must be soft deleted first
    let result = project_service::permanent_delete_project(&pool, project_id).await;
    assert!(result.is_err());

    project_service::soft_delete_project(&pool, project_id).await.unwrap();

    let result = project_service::permanent_delete_project(&pool, project_id).await;
    assert!(result.is_ok());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}
