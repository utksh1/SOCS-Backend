use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::models::application::{Application, ApplicationStatus};

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    experience_level: &str,
    skills: &[String],
    message: Option<&str>,
) -> Result<Application, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (name, email, experience_level, skills, message)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(experience_level)
    .bind(skills)
    .bind(message)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        SELECT id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at
        FROM applications
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        SELECT id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at
        FROM applications
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn review(
    pool: &PgPool,
    id: Uuid,
    status: ApplicationStatus,
    reviewed_by: Uuid,
    rejection_reason: Option<&str>,
) -> Result<Application, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        UPDATE applications 
        SET status = $1, reviewed_by = $2, reviewed_at = $3, rejection_reason = $4, updated_at = NOW()
        WHERE id = $5
        RETURNING id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at
        "#,
    )
    .bind(status)
    .bind(reviewed_by)
    .bind(Utc::now())
    .bind(rejection_reason)
    .bind(id)
    .fetch_one(pool)
    .await
}
