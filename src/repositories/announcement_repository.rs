use sqlx::PgPool;
use uuid::Uuid;
use crate::models::notification::Announcement;

pub async fn create(
    pool: &PgPool,
    title: &str,
    content: &str,
    category: &str,
    pinned: bool,
    author_id: Uuid,
) -> Result<Announcement, sqlx::Error> {
    sqlx::query_as::<_, Announcement>(
        r#"
        INSERT INTO announcements (title, content, category, pinned, author_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, title, content, category, pinned, author_id, created_at, updated_at, deleted_at
        "#
    )
    .bind(title)
    .bind(content)
    .bind(category)
    .bind(pinned)
    .bind(author_id)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Announcement>, sqlx::Error> {
    sqlx::query_as::<_, Announcement>(
        r#"
        SELECT id, title, content, category, pinned, author_id, created_at, updated_at, deleted_at
        FROM announcements
        WHERE deleted_at IS NULL
        ORDER BY pinned DESC, created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Announcement>, sqlx::Error> {
    sqlx::query_as::<_, Announcement>(
        r#"
        SELECT id, title, content, category, pinned, author_id, created_at, updated_at, deleted_at
        FROM announcements
        WHERE id = $1 AND deleted_at IS NULL
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    title: &str,
    content: &str,
    category: &str,
    pinned: bool,
) -> Result<Announcement, sqlx::Error> {
    sqlx::query_as::<_, Announcement>(
        r#"
        UPDATE announcements
        SET title = $2, content = $3, category = $4, pinned = $5, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, title, content, category, pinned, author_id, created_at, updated_at, deleted_at
        "#
    )
    .bind(id)
    .bind(title)
    .bind(content)
    .bind(category)
    .bind(pinned)
    .fetch_one(pool)
    .await
}

pub async fn toggle_pin(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE announcements SET pinned = NOT pinned, updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE announcements SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE announcements SET deleted_at = NULL WHERE id = $1"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM announcements WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
