use sqlx::PgPool;
use uuid::Uuid;

use crate::models::resource::{Resource, ResourceCategory};

pub async fn create(
    pool: &PgPool,
    title: &str,
    description: &str,
    category: ResourceCategory,
    url: &str,
    tags: &[String],
    created_by: Option<Uuid>,
) -> Result<Resource, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        r#"
        INSERT INTO resources (title, description, category, url, tags, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, title, description, category, url, tags, created_by, created_at, updated_at
        "#,
    )
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(url)
    .bind(tags)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        r#"
        SELECT id, title, description, category, url, tags, created_by, created_at, updated_at
        FROM resources
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        r#"
        SELECT id, title, description, category, url, tags, created_by, created_at, updated_at
        FROM resources
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM resources WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
