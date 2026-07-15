use socs_backend::repositories::contact_repository;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_all(pool: PgPool) {
    let email = format!("contact_{}@example.com", Uuid::new_v4());
    
    let contact = contact_repository::create(
        &pool,
        "Test Contact",
        &email,
        "Test Subject",
        "Test Message",
    ).await.unwrap();

    assert_eq!(contact.name, "Test Contact");
    assert_eq!(contact.email, email);
    assert_eq!(contact.subject, "Test Subject");
    assert_eq!(contact.message, "Test Message");
    assert_eq!(contact.replied, false);

    let all = contact_repository::find_all(&pool).await.unwrap();
    assert!(all.iter().any(|c| c.id == contact.id));
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("contact_del_{}@example.com", Uuid::new_v4());
    let contact = contact_repository::create(&pool, "Delete Me", &email, "Subj", "Msg").await.unwrap();

    let deleted = contact_repository::soft_delete(&pool, contact.id).await.unwrap();
    assert!(deleted);

    let all = contact_repository::find_all(&pool).await.unwrap();
    assert!(!all.iter().any(|c| c.id == contact.id));

    let restored = contact_repository::restore(&pool, contact.id).await.unwrap();
    assert!(restored);

    let all_restored = contact_repository::find_all(&pool).await.unwrap();
    assert!(all_restored.iter().any(|c| c.id == contact.id));
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("contact_perm_{}@example.com", Uuid::new_v4());
    let contact = contact_repository::create(&pool, "Perm Delete", &email, "Subj", "Msg").await.unwrap();

    contact_repository::soft_delete(&pool, contact.id).await.unwrap();
    let deleted = contact_repository::permanent_delete(&pool, contact.id).await.unwrap();
    assert!(deleted);

    let all = contact_repository::find_all(&pool).await.unwrap();
    assert!(!all.iter().any(|c| c.id == contact.id));
}
