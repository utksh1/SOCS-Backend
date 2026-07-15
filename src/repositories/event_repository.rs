use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::event::{Event, EventType, EventStatus};

#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    slug: &str,
    title: &str,
    description: &str,
    date: DateTime<Utc>,
    event_type: EventType,
    status: EventStatus,
    location: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (slug, title, description, date, type, status, location, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, slug, title, description, date, type, status, location, created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(title)
    .bind(description)
    .bind(date)
    .bind(event_type)
    .bind(status)
    .bind(location)
    .bind(created_by)
    .fetch_one(pool)
    .await
}



pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT id, slug, title, description, date, type, status, location, created_by, created_at, updated_at
        FROM events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
