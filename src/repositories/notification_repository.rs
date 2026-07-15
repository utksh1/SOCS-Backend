use sqlx::PgPool;
use uuid::Uuid;
use crate::models::notification::Notification;



pub async fn find_by_user(pool: &PgPool, user_id: Uuid, limit: i64) -> Result<Vec<Notification>, sqlx::Error> {
    sqlx::query_as::<_, Notification>(
        r#"
        SELECT id, user_id, title, message, type as notification_type, link, read, created_at, deleted_at
        FROM notifications
        WHERE user_id = $1 AND deleted_at IS NULL
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
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = FALSE AND deleted_at IS NULL"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    Ok(result.0)
}

pub async fn mark_as_read(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn mark_all_as_read(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE notifications SET read = TRUE WHERE user_id = $1 AND read = FALSE AND deleted_at IS NULL"
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn soft_delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notifications SET deleted_at = NOW() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notifications SET deleted_at = NULL WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM notifications WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
