use sqlx::PgPool;


use crate::models::contact::Contact;
use crate::utils::sanitize::normalize_email;

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    subject: &str,
    message: &str,
) -> Result<Contact, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, Contact>(
        r#"
        INSERT INTO contacts (name, email, subject, message)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, email, subject, message, replied, created_at, deleted_at
        "#,
    )
    .bind(name)
    .bind(&email)
    .bind(subject)
    .bind(message)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Contact>, sqlx::Error> {
    sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, name, email, subject, message, replied, created_at, deleted_at
        FROM contacts
        WHERE deleted_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_all_paginated(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Contact>, sqlx::Error> {
    sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, name, email, subject, message, replied, created_at, deleted_at
        FROM contacts
        WHERE deleted_at IS NULL
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn count_all(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM contacts WHERE deleted_at IS NULL")
        .fetch_one(pool)
        .await
}

pub async fn soft_delete(pool: &PgPool, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE contacts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE contacts SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM contacts WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
