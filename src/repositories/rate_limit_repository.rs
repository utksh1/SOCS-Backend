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

    // Check if there's an existing rate limit record within the window
    let existing: Option<(i32,)> = sqlx::query_as(
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

    if let Some((count,)) = existing {
        if count >= max_attempts {
            tracing::warn!(
                email = %email,
                token_type = %token_type.as_str(),
                attempt_count = count,
                max_attempts = max_attempts,
                "Rate limit exceeded"
            );
            return Ok(false);
        }

        // Increment existing counter
        sqlx::query(
            r#"
            UPDATE token_rate_limits
            SET attempt_count = attempt_count + 1
            WHERE email = $1 
            AND token_type = $2 
            AND window_start >= $3
            "#,
        )
        .bind(email)
        .bind(token_type.as_str())
        .bind(window_start)
        .execute(pool)
        .await?;

        tracing::info!(
            email = %email,
            token_type = %token_type.as_str(),
            new_count = count + 1,
            "Rate limit counter incremented"
        );
    } else {
        // Create new rate limit record
        sqlx::query(
            r#"
            INSERT INTO token_rate_limits (email, token_type, attempt_count, window_start)
            VALUES ($1, $2, 1, NOW())
            "#,
        )
        .bind(email)
        .bind(token_type.as_str())
        .execute(pool)
        .await?;

        tracing::info!(
            email = %email,
            token_type = %token_type.as_str(),
            "New rate limit record created"
        );
    }

    Ok(true)
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
