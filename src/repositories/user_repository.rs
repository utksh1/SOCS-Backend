use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::user::User,
    utils::sanitize::normalize_email,
};

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (name, email, password, roles)
        VALUES ($1, $2, $3, ARRAY['MEMBER']::user_role[])
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE LOWER(BTRIM(email)) = $1 AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_profile_picture(
    pool: &PgPool,
    user_id: Uuid,
    profile_picture: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET profile_picture = $1, updated_at = NOW()
        WHERE id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(profile_picture)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn profile_picture_matches(
    pool: &PgPool,
    user_id: Uuid,
    profile_picture: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND profile_picture = $2 AND deleted_at IS NULL)",
    )
    .bind(user_id)
    .bind(profile_picture)
    .fetch_one(pool)
    .await
}

pub async fn clear_profile_picture(
    pool: &PgPool,
    user_id: Uuid,
    profile_picture: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET profile_picture = NULL, updated_at = NOW() WHERE id = $1 AND profile_picture = $2 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(profile_picture)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}

pub async fn find_by_id_with_deleted(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Mark user's email as verified
#[tracing::instrument(name = "mark_email_verified", skip(pool))]
pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET email_verified_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        "User email verified"
    );

    Ok(user)
}

/// Update user's password
#[tracing::instrument(name = "update_password", skip(pool, password_hash))]
pub async fn update_password(
    pool: &PgPool,
    user_id: Uuid,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET password = $1, updated_at = NOW()
        WHERE id = $2 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        "User password updated"
    );

    Ok(user)
}
