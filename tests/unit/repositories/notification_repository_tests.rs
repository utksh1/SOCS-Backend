use socs_backend::repositories::{notification_repository, user_repository};
use socs_backend::models::notification::Notification;
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_test_notification(pool: &PgPool, user_id: Uuid, title: &str, read: bool) -> Notification {
    sqlx::query_as::<_, Notification>(
        r#"
        INSERT INTO notifications (user_id, title, message, type, link, read)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, title, message, type as notification_type, link, read, created_at, deleted_at
        "#
    )
    .bind(user_id)
    .bind(title)
    .bind("Notification Message")
    .bind("alert")
    .bind(None::<String>)
    .bind(read)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn test_find_and_count(pool: PgPool) {
    let email = format!("notif_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Notif User", &email, "hash").await.unwrap();

    let n1 = insert_test_notification(&pool, user.id, "Unread 1", false).await;
    let n2 = insert_test_notification(&pool, user.id, "Unread 2", false).await;
    let n3 = insert_test_notification(&pool, user.id, "Read 1", true).await;

    let unread_count = notification_repository::count_unread(&pool, user.id).await.unwrap();
    assert_eq!(unread_count, 2);

    let all = notification_repository::find_by_user(&pool, user.id, 10).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[sqlx::test]
async fn test_mark_as_read(pool: PgPool) {
    let email = format!("notif_read_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Notif User Read", &email, "hash").await.unwrap();

    let n1 = insert_test_notification(&pool, user.id, "Unread", false).await;
    let n2 = insert_test_notification(&pool, user.id, "Unread", false).await;

    notification_repository::mark_as_read(&pool, n1.id, user.id).await.unwrap();
    
    let unread_count = notification_repository::count_unread(&pool, user.id).await.unwrap();
    assert_eq!(unread_count, 1);

    notification_repository::mark_all_as_read(&pool, user.id).await.unwrap();

    let unread_count_all = notification_repository::count_unread(&pool, user.id).await.unwrap();
    assert_eq!(unread_count_all, 0);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("notif_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Notif User Del", &email, "hash").await.unwrap();

    let n = insert_test_notification(&pool, user.id, "To Delete", false).await;

    let deleted = notification_repository::soft_delete(&pool, n.id, user.id).await.unwrap();
    assert!(deleted);

    let all = notification_repository::find_by_user(&pool, user.id, 10).await.unwrap();
    assert!(all.is_empty());

    let restored = notification_repository::restore(&pool, n.id, user.id).await.unwrap();
    assert!(restored);

    let all_restored = notification_repository::find_by_user(&pool, user.id, 10).await.unwrap();
    assert_eq!(all_restored.len(), 1);
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("notif_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Notif User Perm", &email, "hash").await.unwrap();

    let n = insert_test_notification(&pool, user.id, "To Perm Delete", false).await;

    notification_repository::soft_delete(&pool, n.id, user.id).await.unwrap();
    let deleted = notification_repository::permanent_delete(&pool, n.id, user.id).await.unwrap();
    assert!(deleted);

    let all = notification_repository::find_by_user(&pool, user.id, 10).await.unwrap();
    assert!(all.is_empty());
}
