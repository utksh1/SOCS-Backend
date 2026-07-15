use socs_backend::repositories::{visual_repository, user_repository};
use socs_backend::models::visual::VisualCategory;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_by_id(pool: PgPool) {
    let email = format!("vis_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Vis User", &email, "hash").await.unwrap();

    let visual = visual_repository::create(
        &pool,
        "Test Visual",
        VisualCategory::Team,
        "https://example.com/meme.jpg",
        Some("A funny meme"),
        Some(user.id),
    ).await.unwrap();

    assert_eq!(visual.title, "Test Visual");
    assert_eq!(visual.category, VisualCategory::Team);
    assert_eq!(visual.src, "https://example.com/meme.jpg");
    assert_eq!(visual.alt_text, Some("A funny meme".to_string()));
    assert_eq!(visual.created_by, Some(user.id));

    let found = visual_repository::find_by_id(&pool, visual.id).await.unwrap().unwrap();
    assert_eq!(found.id, visual.id);
}

#[sqlx::test]
async fn test_find_all(pool: PgPool) {
    let email = format!("vis_all_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Vis User All", &email, "hash").await.unwrap();

    let v1 = visual_repository::create(&pool, "V1", VisualCategory::Team, "src1", None, Some(user.id)).await.unwrap();
    let v2 = visual_repository::create(&pool, "V2", VisualCategory::Event, "src2", None, Some(user.id)).await.unwrap();

    let all = visual_repository::find_all(&pool).await.unwrap();
    assert!(all.iter().any(|v| v.id == v1.id));
    assert!(all.iter().any(|v| v.id == v2.id));
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("vis_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Vis User Del", &email, "hash").await.unwrap();

    let visual = visual_repository::create(&pool, "Delete Me", VisualCategory::Team, "src", None, Some(user.id)).await.unwrap();

    let deleted = visual_repository::soft_delete(&pool, visual.id).await.unwrap();
    assert!(deleted);

    let found = visual_repository::find_by_id(&pool, visual.id).await.unwrap();
    assert!(found.is_none());

    // Shouldn't be in find_all either
    let all = visual_repository::find_all(&pool).await.unwrap();
    assert!(!all.iter().any(|v| v.id == visual.id));

    let restored = visual_repository::restore(&pool, visual.id).await.unwrap();
    assert!(restored);

    let found_restored = visual_repository::find_by_id(&pool, visual.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("vis_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Vis User Perm", &email, "hash").await.unwrap();

    let visual = visual_repository::create(&pool, "Perm Delete", VisualCategory::Team, "src", None, Some(user.id)).await.unwrap();

    visual_repository::soft_delete(&pool, visual.id).await.unwrap();
    let deleted = visual_repository::permanent_delete(&pool, visual.id).await.unwrap();
    assert!(deleted);

    let found = visual_repository::find_by_id(&pool, visual.id).await.unwrap();
    assert!(found.is_none());
}
