use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::models::application::{Application, ApplicationStatus};
use crate::utils::sanitize::normalize_email;

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    experience_level: &str,
    skills: &[String],
    message: Option<&str>,
) -> Result<Application, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (name, email, experience_level, skills, message)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at, deleted_at
        "#,
    )
    .bind(name)
    .bind(&email)
    .bind(experience_level)
    .bind(skills)
    .bind(message)
    .fetch_one(pool)
    .await
}



pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Application>, sqlx::Error> {
    sqlx::query_as::<_, Application>(
        r#"
        SELECT id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at, deleted_at
        FROM applications
        WHERE id = $1 AND deleted_at IS NULL
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
        WHERE id = $5 AND deleted_at IS NULL
        RETURNING id, name, email, experience_level, skills, message, status, reviewed_by, reviewed_at, rejection_reason, created_at, updated_at, deleted_at
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

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE applications SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE applications SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM applications WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
