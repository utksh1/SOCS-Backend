use chrono::Utc;
use socs_backend::repositories::{event_registration_repository, event_repository, user_repository};
use socs_backend::models::event::{EventType, EventStatus};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_register_and_find_by_event(pool: PgPool) {
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "User", &email, "hash").await.unwrap();

    let slug = format!("event-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Test Event",
        "Desc",
        Utc::now(),
        EventType::Workshop,
        EventStatus::Upcoming,
        None,
        None,
    ).await.unwrap();

    let reg_email = format!("reg_{}@example.com", Uuid::new_v4());
    let registration = event_registration_repository::register(
        &pool,
        event.id,
        "Attendee",
        &reg_email,
        Some(user.id),
    ).await.unwrap();

    assert_eq!(registration.event_id, event.id);
    assert_eq!(registration.name, "Attendee");
    assert_eq!(registration.email, reg_email);
    assert_eq!(registration.user_id, Some(user.id));
    assert_eq!(registration.attended, false);

    let all = event_registration_repository::find_by_event(&pool, event.id).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, registration.id);

    let count = event_registration_repository::count_by_event(&pool, event.id).await.unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let slug = format!("event-del-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Test Event",
        "Desc",
        Utc::now(),
        EventType::Workshop,
        EventStatus::Upcoming,
        None,
        None,
    ).await.unwrap();

    let reg_email = format!("reg_del_{}@example.com", Uuid::new_v4());
    let registration = event_registration_repository::register(
        &pool,
        event.id,
        "Attendee",
        &reg_email,
        None,
    ).await.unwrap();

    let deleted = event_registration_repository::soft_delete(&pool, registration.id).await.unwrap();
    assert!(deleted);

    let all = event_registration_repository::find_by_event(&pool, event.id).await.unwrap();
    assert!(all.is_empty());

    let count = event_registration_repository::count_by_event(&pool, event.id).await.unwrap();
    assert_eq!(count, 0);

    let restored = event_registration_repository::restore(&pool, registration.id).await.unwrap();
    assert!(restored);

    let all_restored = event_registration_repository::find_by_event(&pool, event.id).await.unwrap();
    assert_eq!(all_restored.len(), 1);
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let slug = format!("event-perm-{}", Uuid::new_v4());
    let event = event_repository::create(
        &pool,
        &slug,
        "Test Event",
        "Desc",
        Utc::now(),
        EventType::Workshop,
        EventStatus::Upcoming,
        None,
        None,
    ).await.unwrap();

    let reg_email = format!("reg_perm_{}@example.com", Uuid::new_v4());
    let registration = event_registration_repository::register(
        &pool,
        event.id,
        "Attendee",
        &reg_email,
        None,
    ).await.unwrap();

    event_registration_repository::soft_delete(&pool, registration.id).await.unwrap();
    let deleted = event_registration_repository::permanent_delete(&pool, registration.id).await.unwrap();
    assert!(deleted);

    let all = event_registration_repository::find_by_event(&pool, event.id).await.unwrap();
    assert!(all.is_empty());
}
