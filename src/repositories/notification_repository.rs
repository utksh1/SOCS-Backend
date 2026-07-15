use sqlx::PgPool;
use uuid::Uuid;
use crate::models::notification::Notification;

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    message: &str,
    notification_type: &str,
    link: Option<&str>,
) -> Result<Notification, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"
        INSERT INTO notifications (user_id, title, message, type, link)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, title, message, type as notification_type, link, read, created_at
        "#
    )
    .bind(user_id)
    .bind(title)
    .bind(message)
    .bind(notification_type)
    .bind(link)
    .fetch_one(pool)
    .await
}

pub async fn find_by_user(pool: &PgPool, user_id: Uuid, limit: i64) -> Result<Vec<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"
        SELECT id, user_id, title, message, type as notification_type, link, read, created_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn count_unread(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = FALSE"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    Ok(result.0)
}

pub async fn mark_as_read(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn mark_all_as_read(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE user_id = $1 AND read = FALSE"
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM notifications WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}
