use chrono::{Utc, Duration};
use socs_backend::repositories::user_repository;
use socs_backend::services::event_service;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_soft_delete_event_cascades(pool: PgPool) {
    let email = format!("event_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Event User", &email, "hash").await.unwrap();

    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO events (id, slug, title, description, date, type, status, location, created_by) VALUES ($1, $2, $3, $4, $5, 'workshop'::event_type, 'upcoming'::event_status, 'http://zoom.us', $6)"
    )
    .bind(event_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Event")
    .bind("Desc")
    .bind(Utc::now() + Duration::days(1))
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    // Insert timeline item
    sqlx::query(
        "INSERT INTO event_timeline_items (event_id, title, time, description, display_order) VALUES ($1, $2, $3, $4, 1)"
    )
    .bind(event_id)
    .bind("Timeline 1")
    .bind("10:00")
    .bind("Description")
    .execute(&pool)
    .await
    .unwrap();

    // Insert prerequisite
    sqlx::query(
        "INSERT INTO event_prerequisites (event_id, title, description) VALUES ($1, $2, $3)"
    )
    .bind(event_id)
    .bind("Prereq 1")
    .bind("Description")
    .execute(&pool)
    .await
    .unwrap();

    // Soft delete
    let result = event_service::soft_delete_event(&pool, event_id).await;
    assert!(result.is_ok());

    // Event should be soft deleted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    // Timeline item should be soft deleted
    let count_timeline: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_timeline_items WHERE event_id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_timeline.0, 0);
    
    // Prereq should be soft deleted
    let count_prereq: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_prerequisites WHERE event_id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_prereq.0, 0);
}

#[sqlx::test]
async fn test_restore_event_cascades(pool: PgPool) {
    let email = format!("event_rest_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Event User", &email, "hash").await.unwrap();

    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO events (id, slug, title, description, date, type, status, location, created_by) VALUES ($1, $2, $3, $4, $5, 'workshop'::event_type, 'upcoming'::event_status, 'http://zoom.us', $6)"
    )
    .bind(event_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Event")
    .bind("Desc")
    .bind(Utc::now() + Duration::days(1))
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO event_timeline_items (event_id, title, time, description, display_order) VALUES ($1, $2, $3, $4, 1)"
    )
    .bind(event_id)
    .bind("Timeline 1")
    .bind("10:00")
    .bind("Description")
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO event_prerequisites (event_id, title, description) VALUES ($1, $2, $3)"
    )
    .bind(event_id)
    .bind("Prereq 1")
    .bind("Description")
    .execute(&pool)
    .await
    .unwrap();

    event_service::soft_delete_event(&pool, event_id).await.unwrap();

    // Restore
    let result = event_service::restore_event(&pool, event_id).await;
    assert!(result.is_ok());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    let count_timeline: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_timeline_items WHERE event_id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_timeline.0, 1);
    
    let count_prereq: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_prerequisites WHERE event_id = $1 AND deleted_at IS NULL")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_prereq.0, 1);
}

#[sqlx::test]
async fn test_permanent_delete_event(pool: PgPool) {
    let email = format!("event_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Event User", &email, "hash").await.unwrap();

    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO events (id, slug, title, description, date, type, status, location, created_by) VALUES ($1, $2, $3, $4, $5, 'workshop'::event_type, 'upcoming'::event_status, 'http://zoom.us', $6)"
    )
    .bind(event_id)
    .bind(format!("slug-{}", Uuid::new_v4()))
    .bind("Test Event")
    .bind("Desc")
    .bind(Utc::now() + Duration::days(1))
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    // Must be soft deleted first
    let result = event_service::permanent_delete_event(&pool, event_id).await;
    assert!(result.is_err());

    event_service::soft_delete_event(&pool, event_id).await.unwrap();

    let result = event_service::permanent_delete_event(&pool, event_id).await;
    assert!(result.is_ok());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}
