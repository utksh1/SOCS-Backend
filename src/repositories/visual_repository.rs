use sqlx::PgPool;
use uuid::Uuid;

use crate::models::visual::{Visual, VisualCategory};

pub async fn create(
    pool: &PgPool,
    title: &str,
    category: VisualCategory,
    src: &str,
    alt_text: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<Visual, sqlx::Error> {
    sqlx::query_as::<_, Visual>(
        r#"
        INSERT INTO visuals (title, category, src, alt_text, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, title, category, src, alt_text, created_by, created_at, updated_at, deleted_at
        "#,
    )
    .bind(title)
    .bind(category)
    .bind(src)
    .bind(alt_text)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Visual>, sqlx::Error> {
    sqlx::query_as::<_, Visual>(
        r#"
        SELECT id, title, category, src, alt_text, created_by, created_at, updated_at, deleted_at
        FROM visuals
        WHERE deleted_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Visual>, sqlx::Error> {
    sqlx::query_as::<_, Visual>(
        r#"
        SELECT id, title, category, src, alt_text, created_by, created_at, updated_at, deleted_at
        FROM visuals
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE visuals SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE visuals SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM visuals WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
