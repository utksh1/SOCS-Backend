use sqlx::PgPool;
use uuid::Uuid;

// Import necessary types from the socs_backend crate
// Note: Since this is a binary crate, we need to ensure modules are accessible

#[sqlx::test]
async fn test_project_soft_delete_and_restore(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user first (needed for created_by)
    let user = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#,
        "Test User",
        "test@example.com",
        "$2b$12$dummy_hash"
    )
    .fetch_one(&pool)
    .await?;

    // Create a test project
    let project = sqlx::query!(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7::content_status, $8)
        RETURNING id, deleted_at
        "#,
        "test-project",
        "Test Project",
        "A test project for soft deletion",
        &vec!["Rust", "PostgreSQL"],
        &vec!["test"],
        false,
        "pending",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(project.deleted_at, None, "Project should not be deleted initially");

    // Perform soft delete
    let soft_delete_result = sqlx::query!(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        project.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(soft_delete_result.rows_affected(), 1, "Soft delete should affect 1 row");

    // Verify project is soft deleted (not visible in normal queries)
    let visible_project = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 AND deleted_at IS NULL",
        project.id
    )
    .fetch_optional(&pool)
    .await?;

    assert_eq!(visible_project, None, "Soft deleted project should not appear in normal queries");

    // Verify project still exists with deleted_at set
    let deleted_project = sqlx::query!(
        "SELECT id, deleted_at FROM projects WHERE id = $1",
        project.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(deleted_project.deleted_at.is_some(), "Project should have deleted_at timestamp");

    // Restore the project
    let restore_result = sqlx::query!(
        "UPDATE projects SET deleted_at = NULL WHERE id = $1",
        project.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(restore_result.rows_affected(), 1, "Restore should affect 1 row");

    // Verify project is visible again
    let restored_project = sqlx::query!(
        "SELECT id, deleted_at FROM projects WHERE id = $1 AND deleted_at IS NULL",
        project.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(restored_project.deleted_at, None, "Restored project should have deleted_at = NULL");

    Ok(())
}

#[sqlx::test]
async fn test_user_soft_delete(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id, deleted_at
        "#,
        "Test User for Soft Delete",
        "softdelete@example.com",
        "$2b$12$dummy_hash"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(user.deleted_at, None, "User should not be deleted initially");

    // Perform soft delete on user
    let soft_delete_result = sqlx::query!(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        user.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(soft_delete_result.rows_affected(), 1, "Soft delete should affect 1 row");

    // Verify user is soft deleted (not visible in normal queries)
    let visible_user = sqlx::query!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL",
        user.id
    )
    .fetch_optional(&pool)
    .await?;

    assert_eq!(visible_user, None, "Soft deleted user should not appear in normal queries");

    // Verify user still exists in database with deleted_at timestamp
    let deleted_user = sqlx::query!(
        "SELECT id, deleted_at FROM users WHERE id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(deleted_user.deleted_at.is_some(), "User should have deleted_at timestamp");

    // Restore the user
    let restore_result = sqlx::query!(
        "UPDATE users SET deleted_at = NULL WHERE id = $1",
        user.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(restore_result.rows_affected(), 1, "Restore should affect 1 row");

    // Verify user is visible again
    let restored_user = sqlx::query!(
        "SELECT id, deleted_at FROM users WHERE id = $1 AND deleted_at IS NULL",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(restored_user.deleted_at, None, "Restored user should have deleted_at = NULL");

    Ok(())
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#,
        "Test User",
        "permanent@example.com",
        "$2b$12$dummy_hash"
    )
    .fetch_one(&pool)
    .await?;

    // Create a test project
    let project = sqlx::query!(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7::content_status, $8)
        RETURNING id
        "#,
        "permanent-delete-test",
        "Permanent Delete Test",
        "A project to test permanent deletion",
        &vec!["Rust"],
        &vec!["test"],
        false,
        "pending",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    // First soft delete the project
    sqlx::query!(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1",
        project.id
    )
    .execute(&pool)
    .await?;

    // Verify project exists (soft deleted)
    let soft_deleted = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1",
        project.id
    )
    .fetch_optional(&pool)
    .await?;

    assert!(soft_deleted.is_some(), "Project should exist after soft delete");

    // Perform permanent delete
    let permanent_delete_result = sqlx::query!(
        "DELETE FROM projects WHERE id = $1",
        project.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(permanent_delete_result.rows_affected(), 1, "Permanent delete should affect 1 row");

    // Verify project no longer exists in database
    let deleted_project = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1",
        project.id
    )
    .fetch_optional(&pool)
    .await?;

    assert_eq!(deleted_project, None, "Project should not exist after permanent delete");

    // Test permanent delete on event
    let event = sqlx::query!(
        r#"
        INSERT INTO events (title, description, location, event_date, registration_deadline)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        "Test Event",
        "Event for permanent deletion test",
        "Test Location",
        chrono::Utc::now() + chrono::Duration::days(30),
        chrono::Utc::now() + chrono::Duration::days(20)
    )
    .fetch_one(&pool)
    .await?;

    // Soft delete first
    sqlx::query!(
        "UPDATE events SET deleted_at = NOW() WHERE id = $1",
        event.id
    )
    .execute(&pool)
    .await?;

    // Permanent delete
    let event_delete_result = sqlx::query!(
        "DELETE FROM events WHERE id = $1",
        event.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(event_delete_result.rows_affected(), 1, "Event permanent delete should affect 1 row");

    // Verify event is gone
    let deleted_event = sqlx::query!(
        "SELECT id FROM events WHERE id = $1",
        event.id
    )
    .fetch_optional(&pool)
    .await?;

    assert_eq!(deleted_event, None, "Event should not exist after permanent delete");

    Ok(())
}

#[sqlx::test]
async fn test_soft_delete_idempotency(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user and project
    let user = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#,
        "Test User",
        "idempotency@example.com",
        "$2b$12$dummy_hash"
    )
    .fetch_one(&pool)
    .await?;

    let project = sqlx::query!(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7::content_status, $8)
        RETURNING id
        "#,
        "idempotency-test",
        "Idempotency Test",
        "Test idempotent soft delete",
        &vec!["Rust"],
        &vec!["test"],
        false,
        "pending",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    // First soft delete
    let first_delete = sqlx::query!(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        project.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(first_delete.rows_affected(), 1, "First soft delete should affect 1 row");

    // Second soft delete (should affect 0 rows since already deleted)
    let second_delete = sqlx::query!(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        project.id
    )
    .execute(&pool)
    .await?;

    assert_eq!(second_delete.rows_affected(), 0, "Second soft delete should affect 0 rows (already deleted)");

    Ok(())
}

#[sqlx::test]
async fn test_soft_delete_cascade_behavior(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user = sqlx::query!(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING id
        "#,
        "Test User",
        "cascade@example.com",
        "$2b$12$dummy_hash"
    )
    .fetch_one(&pool)
    .await?;

    // Create a test project
    let project = sqlx::query!(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7::content_status, $8)
        RETURNING id
        "#,
        "cascade-test",
        "Cascade Test",
        "Test cascade behavior",
        &vec!["Rust"],
        &vec!["test"],
        false,
        "pending",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    // Add a project feature (if the table exists)
    let feature_result = sqlx::query!(
        r#"
        INSERT INTO project_features (project_id, title, description, display_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        project.id,
        "Test Feature",
        "A test feature",
        0
    )
    .fetch_optional(&pool)
    .await;

    // Soft delete the project
    sqlx::query!(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1",
        project.id
    )
    .execute(&pool)
    .await?;

    // Verify project is soft deleted
    let deleted_project = sqlx::query!(
        "SELECT deleted_at FROM projects WHERE id = $1",
        project.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(deleted_project.deleted_at.is_some(), "Project should be soft deleted");

    // If we created a feature, verify it's still accessible (soft delete doesn't cascade automatically)
    if let Ok(Some(feature)) = feature_result {
        let feature_check = sqlx::query!(
            "SELECT id FROM project_features WHERE id = $1",
            feature.id
        )
        .fetch_optional(&pool)
        .await?;

        assert!(feature_check.is_some(), "Project features should remain in database (soft delete doesn't cascade by default)");
    }

    Ok(())
}
