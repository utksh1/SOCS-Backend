use chrono::Utc;
use socs_backend::repositories::{event_repository, user_repository};
use socs_backend::models::event::{EventType, EventStatus};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_event(pool: PgPool) {
    let email = format!("event_user_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Event User", &email, "hash").await.unwrap();

    let slug = format!("test-event-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Test Event",
        "Description",
        Utc::now(),
        EventType::Workshop,
        EventStatus::Upcoming,
        Some("Online"),
        Some(user.id),
    ).await.unwrap();

    assert_eq!(event.title, "Test Event");
    assert_eq!(event.slug, slug);
    assert_eq!(event.event_type, EventType::Workshop);
    assert_eq!(event.status, EventStatus::Upcoming);
    assert_eq!(event.created_by, Some(user.id));

    let found = event_repository::find_by_id(&pool, event.id).await.unwrap().unwrap();
    assert_eq!(found.id, event.id);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let slug = format!("del-event-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Delete Event",
        "Desc",
        Utc::now(),
        EventType::Talk,
        EventStatus::Past,
        None,
        None,
    ).await.unwrap();

    // Soft delete via delete() which calls soft_delete()
    let deleted = event_repository::delete(&pool, event.id).await.unwrap();
    assert!(deleted);

    let found = event_repository::find_by_id(&pool, event.id).await.unwrap();
    assert!(found.is_none());

    let found_deleted = event_repository::find_by_id_with_deleted(&pool, event.id).await.unwrap();
    assert!(found_deleted.is_some());

    let restored = event_repository::restore(&pool, event.id).await.unwrap();
    assert!(restored);

    let found_restored = event_repository::find_by_id(&pool, event.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let slug = format!("perm-del-event-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Perm Delete Event",
        "Desc",
        Utc::now(),
        EventType::Other,
        EventStatus::Cancelled,
        None,
        None,
    ).await.unwrap();

    event_repository::soft_delete(&pool, event.id).await.unwrap();
    let deleted = event_repository::permanent_delete(&pool, event.id).await.unwrap();
    assert!(deleted);

    let found = event_repository::find_by_id_with_deleted(&pool, event.id).await.unwrap();
    assert!(found.is_none());
}
