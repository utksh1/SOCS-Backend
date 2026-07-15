use bcrypt::{hash, DEFAULT_COST};
use chrono::{Duration, Utc};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{
    config::env::Config,
    error::ApiError,
    models::{user::User, verification_token::TokenType},
    repositories::{rate_limit_repository, user_repository, verification_token_repository},
    services::email_service::EmailService,
};

/// Generate a cryptographically secure 32-byte random token as 64-character hex string
#[tracing::instrument(name = "generate_secure_token")]
pub fn generate_secure_token() -> String {
    let mut rng = thread_rng();
    let token_bytes: [u8; 32] = rng.gen();
    hex::encode(token_bytes)
}

/// Produce a deterministic lookup fingerprint for a high-entropy token.
///
/// The token itself is 256 bits of randomness, so a SHA-256 fingerprint is
/// safe to store and allows an indexed equality lookup. Bcrypt is unsuitable
/// here because its random salt produces a different value on every call.
#[tracing::instrument(name = "hash_token", skip(token))]
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Send verification email with rate limiting
/// Rate limit: 3 requests per hour
#[tracing::instrument(
    name = "send_verification_email",
    skip(pool, config, email_service, user),
    fields(user_id = %user.id, user_email = %user.email)
)]
pub async fn send_verification_email(
    pool: &PgPool,
    config: &Config,
    email_service: &EmailService,
    user: &User,
) -> Result<(), ApiError> {
    // Check rate limit: 3 requests per hour
    if !rate_limit_repository::check_and_increment(
        pool,
        &user.email,
        TokenType::EmailVerification,
        3,
        1,
    )
    .await?
    {
        tracing::warn!(
            user_email = %user.email,
            "Email verification rate limit exceeded"
        );
        return Err(ApiError::TooManyRequests(
            "Too many verification emails requested. Please try again later.".to_string(),
        ));
    }

    // Generate secure token
    let token = generate_secure_token();
    let token_hash = hash_token(&token);

    // Create new token with 30-minute expiration
    let expires_at = Utc::now() + Duration::minutes(30);
    let stored_token = verification_token_repository::create(
        pool,
        user.id,
        &token_hash,
        TokenType::EmailVerification,
        expires_at,
    )
    .await?;

    // Generate verification link
    let verification_link = format!(
        "{}/verify-email?token={}",
        config.frontend_url.trim_end_matches('/'),
        token
    );

    // Do not invalidate an older usable link until the replacement email has
    // been handed to SMTP. If delivery fails, remove only the new orphaned
    // token so the existing link remains usable.
    let delivery_error = email_service
        .send_verification_email(&user.email, &user.name, &verification_link)
        .await
        .err()
        .map(|error| error.to_string());
    if let Some(error) = delivery_error {
        if let Err(cleanup_error) = sqlx::query("DELETE FROM verification_tokens WHERE id = $1")
            .bind(stored_token.id)
            .execute(pool)
            .await
        {
            tracing::error!(%cleanup_error, token_id = %stored_token.id, "Failed to remove undelivered verification token");
        }
        tracing::error!(%error, user_email = %user.email, "Failed to send verification email");
        return Err(ApiError::ServiceUnavailable(
            "Email delivery is temporarily unavailable".to_string(),
        ));
    }

    verification_token_repository::delete_other_user_tokens(
        pool,
        user.id,
        TokenType::EmailVerification,
        stored_token.id,
    )
    .await?;

    tracing::info!(
        user_id = %user.id,
        user_email = %user.email,
        "Verification email sent successfully"
    );

    Ok(())
}

/// Verify email using token and mark email as verified
#[tracing::instrument(name = "verify_email_token", skip(pool, token_string))]
pub async fn verify_email_token(
    pool: &PgPool,
    token_string: &str,
) -> Result<User, ApiError> {
    // Hash the provided token to compare with database
    let token_hash = hash_token(token_string);

    // Find token by hash
    let stored_token = verification_token_repository::find_by_hash(pool, &token_hash)
        .await?
        .ok_or_else(|| {
            tracing::warn!("Invalid or non-existent verification token");
            ApiError::BadRequest("Invalid or expired verification token".to_string())
        })?;

    // Verify token type
    if stored_token.token_type != TokenType::EmailVerification {
        tracing::warn!(
            token_id = %stored_token.id,
            expected_type = "email_verification",
            actual_type = %stored_token.token_type.as_str(),
            "Token type mismatch"
        );
        return Err(ApiError::BadRequest("Invalid token type".to_string()));
    }

    // Check if token is valid (not expired and not used)
    if !stored_token.is_valid() {
        if stored_token.is_used() {
            tracing::warn!(
                token_id = %stored_token.id,
                "Token already used"
            );
            return Err(ApiError::BadRequest("Token has already been used".to_string()));
        }
        if stored_token.is_expired() {
            tracing::warn!(
                token_id = %stored_token.id,
                "Token expired"
            );
            return Err(ApiError::BadRequest("Token has expired".to_string()));
        }
        return Err(ApiError::BadRequest("Invalid token".to_string()));
    }

    // Claiming the token and updating the account must be one transaction. If
    // the account update fails, the token remains usable rather than being
    // consumed without completing verification.
    let mut transaction = pool.begin().await?;
    let claim = sqlx::query(
        "UPDATE verification_tokens SET used_at = NOW() WHERE id = $1 AND used_at IS NULL AND expires_at > NOW()",
    )
    .bind(stored_token.id)
    .execute(&mut *transaction)
    .await?;
    if claim.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Invalid or expired verification token".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL RETURNING *",
    )
    .bind(stored_token.user_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
    transaction.commit().await?;

    tracing::info!(
        user_id = %user.id,
        user_email = %user.email,
        "Email verified successfully"
    );

    Ok(user)
}

/// Send password reset email with rate limiting
/// Rate limit: 3 requests per hour
/// Returns success even if email doesn't exist (prevents enumeration)
#[tracing::instrument(
    name = "send_password_reset_email",
    skip(pool, config, email_service, email)
)]
pub async fn send_password_reset_email(
    pool: &PgPool,
    config: &Config,
    email_service: &EmailService,
    email: &str,
) -> Result<(), ApiError> {
    let email = crate::utils::sanitize::normalize_email(email);

    // Check rate limit: 3 requests per hour
    if !rate_limit_repository::check_and_increment(
        pool,
        &email,
        TokenType::PasswordReset,
        3,
        1,
    )
    .await?
    {
        tracing::warn!(
            email = %email,
            "Password reset rate limit exceeded"
        );
        return Err(ApiError::TooManyRequests(
            "Too many password reset requests. Please try again later.".to_string(),
        ));
    }

    // Find user by email (return success even if not found to prevent enumeration)
    let user = match user_repository::find_by_email(pool, &email).await? {
        Some(user) => user,
        None => {
            tracing::info!(
                email = %email,
                "Password reset requested for non-existent email (returning success to prevent enumeration)"
            );
            // Return success to prevent email enumeration
            return Ok(());
        }
    };

    // Generate secure token
    let token = generate_secure_token();
    let token_hash = hash_token(&token);

    // Create new token with 30-minute expiration
    let expires_at = Utc::now() + Duration::minutes(30);
    let stored_token = verification_token_repository::create(
        pool,
        user.id,
        &token_hash,
        TokenType::PasswordReset,
        expires_at,
    )
    .await?;

    // Generate reset link
    let reset_link = format!(
        "{}/reset-password?token={}",
        config.frontend_url.trim_end_matches('/'),
        token
    );

    let delivery_error = email_service
        .send_password_reset_email(&user.email, &user.name, &reset_link)
        .await
        .err()
        .map(|error| error.to_string());
    if let Some(error) = delivery_error {
        if let Err(cleanup_error) = sqlx::query("DELETE FROM verification_tokens WHERE id = $1")
            .bind(stored_token.id)
            .execute(pool)
            .await
        {
            tracing::error!(%cleanup_error, token_id = %stored_token.id, "Failed to remove undelivered password-reset token");
        }
        tracing::error!(%error, user_email = %user.email, "Failed to send password reset email");
        return Err(ApiError::ServiceUnavailable(
            "Email delivery is temporarily unavailable".to_string(),
        ));
    }

    verification_token_repository::delete_other_user_tokens(
        pool,
        user.id,
        TokenType::PasswordReset,
        stored_token.id,
    )
    .await?;

    tracing::info!(
        user_id = %user.id,
        user_email = %user.email,
        "Password reset email sent successfully"
    );

    Ok(())
}

/// Reset password using token
#[tracing::instrument(
    name = "reset_password_with_token",
    skip(pool, email_service, token_string, new_password)
)]
pub async fn reset_password_with_token(
    pool: &PgPool,
    email_service: &EmailService,
    token_string: &str,
    new_password: &str,
) -> Result<User, ApiError> {
    // Validate password strength (minimum 8 characters)
    if new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    // Hash the provided token to compare with database
    let token_hash = hash_token(token_string);

    // Find token by hash
    let stored_token = verification_token_repository::find_by_hash(pool, &token_hash)
        .await?
        .ok_or_else(|| {
            tracing::warn!("Invalid or non-existent password reset token");
            ApiError::BadRequest("Invalid or expired password reset token".to_string())
        })?;

    // Verify token type
    if stored_token.token_type != TokenType::PasswordReset {
        tracing::warn!(
            token_id = %stored_token.id,
            expected_type = "password_reset",
            actual_type = %stored_token.token_type.as_str(),
            "Token type mismatch"
        );
        return Err(ApiError::BadRequest("Invalid token type".to_string()));
    }

    // Check if token is valid (not expired and not used)
    if !stored_token.is_valid() {
        if stored_token.is_used() {
            tracing::warn!(
                token_id = %stored_token.id,
                "Token already used"
            );
            return Err(ApiError::BadRequest("Token has already been used".to_string()));
        }
        if stored_token.is_expired() {
            tracing::warn!(
                token_id = %stored_token.id,
                "Token expired"
            );
            return Err(ApiError::BadRequest("Token has expired".to_string()));
        }
        return Err(ApiError::BadRequest("Invalid token".to_string()));
    }

    // Hash the new password in a blocking task
    let new_password_clone = new_password.to_string();
    let password_hash = tokio::task::spawn_blocking(move || {
        hash(&new_password_clone, DEFAULT_COST)
    })
    .await
    .map_err(|_| ApiError::InternalServerError)?
    .map_err(|_| ApiError::InternalServerError)?;

    // Consume the token and update the password atomically. A database failure
    // must not invalidate the only reset link without changing the password.
    let mut transaction = pool.begin().await?;
    let claim = sqlx::query(
        "UPDATE verification_tokens SET used_at = NOW() WHERE id = $1 AND used_at IS NULL AND expires_at > NOW()",
    )
    .bind(stored_token.id)
    .execute(&mut *transaction)
    .await?;
    if claim.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Invalid or expired password reset token".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET password = $1, token_version = token_version + 1, updated_at = NOW() WHERE id = $2 AND deleted_at IS NULL RETURNING *",
    )
    .bind(&password_hash)
    .bind(stored_token.user_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // A successful reset invalidates every remaining reset link, including
    // links that may have been generated concurrently.
    sqlx::query(
        "DELETE FROM verification_tokens WHERE user_id = $1 AND token_type = 'password_reset'",
    )
    .bind(stored_token.user_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    // Send confirmation email
    if let Err(e) = email_service
        .send_password_reset_confirmation(&user.email, &user.name)
        .await
    {
        tracing::error!(
            error = %e,
            user_email = %user.email,
            "Failed to send password reset confirmation email"
        );
        // Don't fail the request if confirmation email fails
    }

    tracing::info!(
        user_id = %user.id,
        user_email = %user.email,
        "Password reset successfully"
    );

    Ok(user)
}

/// Clean up expired tokens and old rate limits
/// Should be called periodically (e.g., via a scheduled job)
#[tracing::instrument(name = "cleanup_expired_data", skip(pool))]
pub async fn cleanup_expired_data(pool: &PgPool) -> Result<(), ApiError> {
    // Clean up expired and used verification tokens
    let tokens_deleted = verification_token_repository::cleanup_expired(pool).await?;

    // Clean up rate limits older than 24 hours
    let rate_limits_deleted = rate_limit_repository::cleanup_old_records(pool, 24).await?;

    tracing::info!(
        tokens_deleted = tokens_deleted,
        rate_limits_deleted = rate_limits_deleted,
        "Expired data cleanup completed"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{generate_secure_token, hash_token};

    #[test]
    fn token_fingerprint_is_stable_and_does_not_reveal_the_token() {
        let token = generate_secure_token();
        let fingerprint = hash_token(&token);

        assert_eq!(fingerprint, hash_token(&token));
        assert_ne!(fingerprint, token);
        assert_eq!(fingerprint.len(), 64);
    }
}
