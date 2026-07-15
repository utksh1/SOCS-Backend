use chrono::{Duration, Utc};
use sqlx::PgPool;

use crate::models::verification_token::TokenType;

/// Check rate limit and increment counter if allowed
/// Returns true if request is allowed, false if rate limit exceeded
#[tracing::instrument(name = "check_rate_limit", skip(pool))]
pub async fn check_and_increment(
    pool: &PgPool,
    email: &str,
    token_type: TokenType,
    max_attempts: i32,
    window_hours: i32,
) -> Result<bool, sqlx::Error> {
    // Calculate window start time
    let window_start = Utc::now() - Duration::hours(window_hours as i64);
    
    // Clean up old records for this email/type combination first
    sqlx::query(
        r#"
        DELETE FROM token_rate_limits
        WHERE email = $1 
        AND token_type = $2 
        AND window_start < $3
        "#,
    )
    .bind(email)
    .bind(token_type.as_str())
    .bind(window_start)
    .execute(pool)
    .await?;

    // Atomic upsert with constraint check
    // This ensures only one record exists per email+token_type+window_start combination
    let result: Result<(i32,), sqlx::Error> = sqlx::query_as(
        r#"
        INSERT INTO token_rate_limits (email, token_type, attempt_count, window_start)
        VALUES ($1, $2, 1, $3)
        ON CONFLICT (email, token_type, window_start)
        DO UPDATE SET attempt_count = token_rate_limits.attempt_count + 1
        WHERE token_rate_limits.attempt_count < $4
        RETURNING attempt_count
        "#,
    )
    .bind(email)
    .bind(token_type.as_str())
    .bind(window_start)
    .bind(max_attempts)
    .fetch_one(pool)
    .await;

    match result {
        Ok((new_count,)) => {
            tracing::info!(
                email = %email,
                token_type = %token_type.as_str(),
                attempt_count = new_count,
                max_attempts = max_attempts,
                "Rate limit check passed, counter incremented"
            );
            Ok(true)
        }
        Err(sqlx::Error::RowNotFound) => {
            // The WHERE clause failed, meaning we're at the limit
            tracing::warn!(
                email = %email,
                token_type = %token_type.as_str(),
                max_attempts = max_attempts,
                "Rate limit exceeded"
            );
            Ok(false)
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                email = %email,
                token_type = %token_type.as_str(),
                "Rate limit check failed"
            );
            Err(e)
        }
    }
}

/// Clean up old rate limit records (for scheduled cleanup)
#[tracing::instrument(name = "cleanup_rate_limits", skip(pool))]
pub async fn cleanup_old_records(
    pool: &PgPool,
    older_than_hours: i32,
) -> Result<u64, sqlx::Error> {
    let cutoff_time = Utc::now() - Duration::hours(older_than_hours as i64);

    let result = sqlx::query(
        r#"
        DELETE FROM token_rate_limits
        WHERE window_start < $1
        "#,
    )
    .bind(cutoff_time)
    .execute(pool)
    .await?;

    let rows_affected = result.rows_affected();

    tracing::info!(
        rows_deleted = rows_affected,
        older_than_hours = older_than_hours,
        "Old rate limit records cleaned up"
    );

    Ok(rows_affected)
}

/// Get current attempt count for an email and token type
#[tracing::instrument(name = "get_attempt_count", skip(pool))]
pub async fn get_attempt_count(
    pool: &PgPool,
    email: &str,
    token_type: TokenType,
    window_hours: i32,
) -> Result<i32, sqlx::Error> {
    let window_start = Utc::now() - Duration::hours(window_hours as i64);

    let count: Option<(i32,)> = sqlx::query_as(
        r#"
        SELECT attempt_count FROM token_rate_limits
        WHERE email = $1 
        AND token_type = $2 
        AND window_start >= $3
        "#,
    )
    .bind(email)
    .bind(token_type.as_str())
    .bind(window_start)
    .fetch_optional(pool)
    .await?;

    Ok(count.map(|(c,)| c).unwrap_or(0))
}
