use sqlx::PgPool;
use socs_backend::services::project_service;

// Integration tests for soft deletion functionality
// Note: These tests use sqlx::test which automatically sets up and tears down test databases

#[sqlx::test]
async fn test_project_soft_delete_and_restore(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user first (needed for created_by)
    let user_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#
    )
    .bind("Test User")
    .bind("test@example.com")
    .bind("$2b$12$dummy_hash")
    .fetch_one(&pool)
    .await?;

    // Create a test project using query_scalar to avoid type issues
    let project_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending'::content_status, $7)
        RETURNING id
        "#
    )
    .bind("test-project")
    .bind("Test Project")
    .bind("A test project for soft deletion")
    .bind(vec!["Rust", "PostgreSQL"])
    .bind(vec!["test"])
    .bind(false)
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Verify project is not deleted initially
    let initial_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(initial_deleted_at, None, "Project should not be deleted initially");

    // Perform soft delete
    let soft_delete_result = sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    assert_eq!(soft_delete_result.rows_affected(), 1, "Soft delete should affect 1 row");

    // Verify project is soft deleted (not visible in normal queries)
    let visible_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(visible_count, 0, "Soft deleted project should not appear in normal queries");

    // Verify project still exists with deleted_at set
    let deleted_at_check: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert!(deleted_at_check.is_some(), "Project should have deleted_at timestamp");

    // Restore the project
    let restore_result = sqlx::query(
        "UPDATE projects SET deleted_at = NULL WHERE id = $1"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    assert_eq!(restore_result.rows_affected(), 1, "Restore should affect 1 row");

    // Verify project is visible again
    let restored_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM projects WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(restored_deleted_at, None, "Restored project should have deleted_at = NULL");

    Ok(())
}

#[sqlx::test]
async fn test_user_soft_delete(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#
    )
    .bind("Test User for Soft Delete")
    .bind("softdelete@example.com")
    .bind("$2b$12$dummy_hash")
    .fetch_one(&pool)
    .await?;

    // Verify user is not deleted initially
    let initial_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(initial_deleted_at, None, "User should not be deleted initially");

    // Perform soft delete on user
    let soft_delete_result = sqlx::query(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(user_id)
    .execute(&pool)
    .await?;

    assert_eq!(soft_delete_result.rows_affected(), 1, "Soft delete should affect 1 row");

    // Verify user is soft deleted (not visible in normal queries)
    let visible_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(visible_count, 0, "Soft deleted user should not appear in normal queries");

    // Verify user still exists in database with deleted_at timestamp
    let deleted_at_check: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    assert!(deleted_at_check.is_some(), "User should have deleted_at timestamp");

    // Restore the user
    let restore_result = sqlx::query(
        "UPDATE users SET deleted_at = NULL WHERE id = $1"
    )
    .bind(user_id)
    .execute(&pool)
    .await?;

    assert_eq!(restore_result.rows_affected(), 1, "Restore should affect 1 row");

    // Verify user is visible again
    let restored_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM users WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(restored_deleted_at, None, "Restored user should have deleted_at = NULL");

    Ok(())
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#
    )
    .bind("Test User")
    .bind("permanent@example.com")
    .bind("$2b$12$dummy_hash")
    .fetch_one(&pool)
    .await?;

    // Create a test project
    let project_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending'::content_status, $7)
        RETURNING id
        "#
    )
    .bind("permanent-delete-test")
    .bind("Permanent Delete Test")
    .bind("A project to test permanent deletion")
    .bind(vec!["Rust"])
    .bind(vec!["test"])
    .bind(false)
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // First soft delete the project
    sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    // Verify project exists (soft deleted)
    let soft_deleted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(soft_deleted_count, 1, "Project should exist after soft delete");

    // Perform permanent delete
    let permanent_delete_result = sqlx::query(
        "DELETE FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    assert_eq!(permanent_delete_result.rows_affected(), 1, "Permanent delete should affect 1 row");

    // Verify project no longer exists in database
    let deleted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(deleted_count, 0, "Project should not exist after permanent delete");

    // Test permanent delete on event
    let event_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO events (slug, title, description, date, type, status)
        VALUES ($1, $2, $3, $4, 'workshop'::event_type, 'upcoming'::event_status)
        RETURNING id
        "#
    )
    .bind("test-event")
    .bind("Test Event")
    .bind("Event for permanent deletion test")
    .bind(chrono::Utc::now() + chrono::Duration::days(30))
    .fetch_one(&pool)
    .await?;

    // Soft delete first
    sqlx::query(
        "UPDATE events SET deleted_at = NOW() WHERE id = $1"
    )
    .bind(event_id)
    .execute(&pool)
    .await?;

    // Permanent delete
    let event_delete_result = sqlx::query(
        "DELETE FROM events WHERE id = $1"
    )
    .bind(event_id)
    .execute(&pool)
    .await?;

    assert_eq!(event_delete_result.rows_affected(), 1, "Event permanent delete should affect 1 row");

    // Verify event is gone
    let event_deleted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events WHERE id = $1"
    )
    .bind(event_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(event_deleted_count, 0, "Event should not exist after permanent delete");

    Ok(())
}

#[sqlx::test]
async fn test_soft_delete_idempotency(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user and project
    let user_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#
    )
    .bind("Test User")
    .bind("idempotency@example.com")
    .bind("$2b$12$dummy_hash")
    .fetch_one(&pool)
    .await?;

    let project_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending'::content_status, $7)
        RETURNING id
        "#
    )
    .bind("idempotency-test")
    .bind("Idempotency Test")
    .bind("Test idempotent soft delete")
    .bind(vec!["Rust"])
    .bind(vec!["test"])
    .bind(false)
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // First soft delete
    let first_delete = sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    assert_eq!(first_delete.rows_affected(), 1, "First soft delete should affect 1 row");

    // Second soft delete (should affect 0 rows since already deleted)
    let second_delete = sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    assert_eq!(second_delete.rows_affected(), 0, "Second soft delete should affect 0 rows (already deleted)");

    Ok(())
}

#[sqlx::test]
async fn test_soft_delete_cascade_behavior(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#
    )
    .bind("Test User")
    .bind("cascade@example.com")
    .bind("$2b$12$dummy_hash")
    .fetch_one(&pool)
    .await?;

    // Create a test project
    let project_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending'::content_status, $7)
        RETURNING id
        "#
    )
    .bind("cascade-test")
    .bind("Cascade Test")
    .bind("Test cascade behavior")
    .bind(vec!["Rust"])
    .bind(vec!["test"])
    .bind(false)
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Add a project feature; it is intrinsic content and must be cascaded by
    // the service transaction rather than exposed after its parent is deleted.
    let feature_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO project_features (project_id, title, description, display_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#
    )
    .bind(project_id)
    .bind("Test Feature")
    .bind("A test feature")
    .bind(0i32)
    .fetch_one(&pool)
    .await?;

    project_service::soft_delete_project(&pool, project_id)
        .await
        .expect("project soft delete should cascade to intrinsic content");

    // Verify project is soft deleted
    let deleted_at_check: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    assert!(deleted_at_check.is_some(), "Project should be soft deleted");

    let feature_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM project_features WHERE id = $1"
    )
    .bind(feature_id)
    .fetch_one(&pool)
    .await?;
    assert!(feature_deleted_at.is_some(), "Project features must be soft deleted with the project");

    project_service::restore_project(&pool, project_id)
        .await
        .expect("project restore should restore intrinsic content");
    let restored_feature_deleted_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT deleted_at FROM project_features WHERE id = $1"
    )
    .bind(feature_id)
    .fetch_one(&pool)
    .await?;
    assert!(restored_feature_deleted_at.is_none(), "Project restore must restore its features");

    Ok(())
}
