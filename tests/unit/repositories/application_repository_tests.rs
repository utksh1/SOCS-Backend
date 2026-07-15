use socs_backend::repositories::{application_repository, user_repository};
use socs_backend::models::application::ApplicationStatus;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_by_id(pool: PgPool) {
    let email = format!("app_{}@example.com", Uuid::new_v4());
    
    let application = application_repository::create(
        &pool,
        "Test Applicant",
        &email,
        "Intermediate",
        &["Rust".to_string(), "Postgres".to_string()],
        Some("I want to join!"),
    ).await.unwrap();

    assert_eq!(application.name, "Test Applicant");
    assert_eq!(application.email, email);
    assert_eq!(application.status, ApplicationStatus::Pending);

    let found = application_repository::find_by_id(&pool, application.id).await.unwrap().unwrap();
    assert_eq!(found.id, application.id);
}

#[sqlx::test]
async fn test_review(pool: PgPool) {
    let email = format!("app_rev_{}@example.com", Uuid::new_v4());
    let application = application_repository::create(&pool, "Review Me", &email, "Beginner", &[], None).await.unwrap();
    
    let user_email = format!("reviewer_{}@example.com", Uuid::new_v4());
    let reviewer = user_repository::create(&pool, "Reviewer", &user_email, "hash").await.unwrap();

    let reviewed = application_repository::review(
        &pool,
        application.id,
        ApplicationStatus::Rejected,
        reviewer.id,
        Some("Not enough experience"),
    ).await.unwrap();

    assert_eq!(reviewed.status, ApplicationStatus::Rejected);
    assert_eq!(reviewed.reviewed_by, Some(reviewer.id));
    assert_eq!(reviewed.rejection_reason, Some("Not enough experience".to_string()));
    assert!(reviewed.reviewed_at.is_some());
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("app_del_{}@example.com", Uuid::new_v4());
    let application = application_repository::create(&pool, "Delete Me", &email, "Adv", &[], None).await.unwrap();

    let deleted = application_repository::soft_delete(&pool, application.id).await.unwrap();
    assert!(deleted);

    let found = application_repository::find_by_id(&pool, application.id).await.unwrap();
    assert!(found.is_none());

    let restored = application_repository::restore(&pool, application.id).await.unwrap();
    assert!(restored);

    let found_restored = application_repository::find_by_id(&pool, application.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("app_perm_{}@example.com", Uuid::new_v4());
    let application = application_repository::create(&pool, "Perm Delete", &email, "Adv", &[], None).await.unwrap();

    application_repository::soft_delete(&pool, application.id).await.unwrap();
    let deleted = application_repository::permanent_delete(&pool, application.id).await.unwrap();
    assert!(deleted);

    let found = application_repository::find_by_id(&pool, application.id).await.unwrap();
    assert!(found.is_none());
}
