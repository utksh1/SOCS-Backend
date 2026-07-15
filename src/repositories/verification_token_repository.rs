use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::verification_token::{TokenType, VerificationToken};

/// Create a new verification token
#[tracing::instrument(name = "create_verification_token", skip(pool, token_hash))]
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    token_type: TokenType,
    expires_at: DateTime<Utc>,
) -> Result<VerificationToken, sqlx::Error> {
    let token = sqlx::query_as::<_, VerificationToken>(
        r#"
        INSERT INTO verification_tokens (user_id, token_hash, token_type, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(token_type.as_str())
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        token_id = %token.id,
        user_id = %user_id,
        token_type = %token_type.as_str(),
        "Verification token created"
    );

    Ok(token)
}

/// Find a token by its hash
#[tracing::instrument(name = "find_token_by_hash", skip(pool, token_hash))]
pub async fn find_by_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<VerificationToken>, sqlx::Error> {
    sqlx::query_as::<_, VerificationToken>(
        r#"
        SELECT * FROM verification_tokens
        WHERE token_hash = $1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

/// Mark a token as used
#[tracing::instrument(name = "mark_token_used", skip(pool))]
pub async fn mark_as_used(
    pool: &PgPool,
    token_id: Uuid,
) -> Result<Option<VerificationToken>, sqlx::Error> {
    let token = sqlx::query_as::<_, VerificationToken>(
        r#"
        UPDATE verification_tokens
        SET used_at = NOW()
        WHERE id = $1 AND used_at IS NULL AND expires_at > NOW()
        RETURNING *
        "#,
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await?;

    if token.is_some() {
        tracing::info!(token_id = %token_id, "Token marked as used");
    }

    Ok(token)
}

/// Delete all tokens for a user of a specific type
#[tracing::instrument(name = "delete_user_tokens", skip(pool))]
pub async fn delete_user_tokens(
    pool: &PgPool,
    user_id: Uuid,
    token_type: TokenType,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM verification_tokens
        WHERE user_id = $1 AND token_type = $2
        "#,
    )
    .bind(user_id)
    .bind(token_type.as_str())
    .execute(pool)
    .await?;

    let rows_affected = result.rows_affected();
    
    tracing::info!(
        user_id = %user_id,
        token_type = %token_type.as_str(),
        rows_deleted = rows_affected,
        "User tokens deleted"
    );

    Ok(rows_affected)
}

/// Remove every token of a type except the one that was just delivered.
/// Keeping the previously delivered token until the replacement email has
/// actually been accepted by SMTP avoids locking a user out on a mail outage.
#[tracing::instrument(name = "delete_other_user_tokens", skip(pool))]
pub async fn delete_other_user_tokens(
    pool: &PgPool,
    user_id: Uuid,
    token_type: TokenType,
    keep_token_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM verification_tokens
        WHERE user_id = $1 AND token_type = $2 AND id <> $3
        "#,
    )
    .bind(user_id)
    .bind(token_type.as_str())
    .bind(keep_token_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Clean up expired and used tokens (for scheduled cleanup)
#[tracing::instrument(name = "cleanup_expired_tokens", skip(pool))]
pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM verification_tokens
        WHERE expires_at < NOW() OR used_at IS NOT NULL
        "#,
    )
    .execute(pool)
    .await?;

    let rows_affected = result.rows_affected();
    
    tracing::info!(
        rows_deleted = rows_affected,
        "Expired and used tokens cleaned up"
    );

    Ok(rows_affected)
}

/// Count active tokens for a user of a specific type
#[tracing::instrument(name = "count_active_tokens", skip(pool))]
pub async fn count_active_tokens(
    pool: &PgPool,
    user_id: Uuid,
    token_type: TokenType,
) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM verification_tokens
        WHERE user_id = $1 
        AND token_type = $2 
        AND expires_at > NOW() 
        AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .bind(token_type.as_str())
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}
