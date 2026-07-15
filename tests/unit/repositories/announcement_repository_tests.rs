use socs_backend::repositories::{announcement_repository, user_repository};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_all(pool: PgPool) {
    let email = format!("ann_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Ann User", &email, "hash").await.unwrap();

    let a1 = announcement_repository::create(
        &pool,
        "Normal Announcement",
        "Content",
        "General",
        false,
        user.id,
    ).await.unwrap();

    let a2 = announcement_repository::create(
        &pool,
        "Pinned Announcement",
        "Content",
        "Important",
        true,
        user.id,
    ).await.unwrap();

    let all = announcement_repository::find_all(&pool).await.unwrap();
    // Pinned should be first
    assert!(all.len() >= 2);
    let pinned_idx = all.iter().position(|a| a.id == a2.id).unwrap();
    let normal_idx = all.iter().position(|a| a.id == a1.id).unwrap();
    
    assert!(pinned_idx < normal_idx); // Because ORDER BY pinned DESC
}

#[sqlx::test]
async fn test_update_and_toggle_pin(pool: PgPool) {
    let email = format!("ann_upd_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Ann User Upd", &email, "hash").await.unwrap();

    let ann = announcement_repository::create(
        &pool,
        "Title",
        "Content",
        "Gen",
        false,
        user.id,
    ).await.unwrap();

    let updated = announcement_repository::update(
        &pool,
        ann.id,
        "New Title",
        "New Content",
        "New Cat",
        false,
    ).await.unwrap();

    assert_eq!(updated.title, "New Title");

    announcement_repository::toggle_pin(&pool, ann.id).await.unwrap();
    
    let found = announcement_repository::find_by_id(&pool, ann.id).await.unwrap().unwrap();
    assert!(found.pinned);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("ann_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Ann User Del", &email, "hash").await.unwrap();

    let ann = announcement_repository::create(&pool, "Title", "Content", "Gen", false, user.id).await.unwrap();

    let deleted = announcement_repository::soft_delete(&pool, ann.id).await.unwrap();
    assert!(deleted);

    let found = announcement_repository::find_by_id(&pool, ann.id).await.unwrap();
    assert!(found.is_none());

    let restored = announcement_repository::restore(&pool, ann.id).await.unwrap();
    assert!(restored);

    let found_restored = announcement_repository::find_by_id(&pool, ann.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("ann_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Ann User Perm", &email, "hash").await.unwrap();

    let ann = announcement_repository::create(&pool, "Title", "Content", "Gen", false, user.id).await.unwrap();

    announcement_repository::soft_delete(&pool, ann.id).await.unwrap();
    let deleted = announcement_repository::permanent_delete(&pool, ann.id).await.unwrap();
    assert!(deleted);

    let found = announcement_repository::find_by_id(&pool, ann.id).await.unwrap();
    assert!(found.is_none());
}
