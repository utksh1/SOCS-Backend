use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::event_registration::EventRegistration;

pub async fn register(
    pool: &PgPool,
    event_id: Uuid,
    name: &str,
    email: &str,
    user_id: Option<Uuid>,
) -> Result<EventRegistration, sqlx::Error> {
    sqlx::query_as::<_, EventRegistration>(
        r#"
        INSERT INTO event_registrations (event_id, name, email, user_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, event_id, name, email, user_id, attended, created_at
        "#,
    )
    .bind(event_id)
    .bind(name)
    .bind(email)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn find_by_event(pool: &PgPool, event_id: Uuid) -> Result<Vec<EventRegistration>, sqlx::Error> {
    sqlx::query_as::<_, EventRegistration>(
        r#"
        SELECT id, event_id, name, email, user_id, attended, created_at
        FROM event_registrations
        WHERE event_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(event_id)
    .fetch_all(pool)
    .await
}

pub async fn count_by_event(pool: &PgPool, event_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM event_registrations WHERE event_id = $1")
        .bind(event_id)
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}
