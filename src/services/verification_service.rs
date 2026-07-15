use bcrypt::{hash, DEFAULT_COST};
use chrono::{Duration, Utc};
use rand::{thread_rng, Rng};
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

/// Hash a token using bcrypt with cost factor 12
#[tracing::instrument(name = "hash_token", skip(token))]
pub fn hash_token(token: &str) -> Result<String, bcrypt::BcryptError> {
    hash(token, DEFAULT_COST)
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
    let token_hash = hash_token(&token)?;

    // Delete any existing verification tokens for this user
    verification_token_repository::delete_user_tokens(
        pool,
        user.id,
        TokenType::EmailVerification,
    )
    .await?;

    // Create new token with 30-minute expiration
    let expires_at = Utc::now() + Duration::minutes(30);
    verification_token_repository::create(
        pool,
        user.id,
        &token_hash,
        TokenType::EmailVerification,
        expires_at,
    )
    .await?;

    // Generate verification link
    let verification_link = format!("{}/verify-email?token={}", config.frontend_url, token);

    // Send email
    email_service
        .send_verification_email(&user.email, &user.name, &verification_link)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_email = %user.email,
                "Failed to send verification email"
            );
            ApiError::InternalServerError
        })?;

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
    let token_hash = hash_token(token_string)?;

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

    // Mark token as used
    verification_token_repository::mark_as_used(pool, stored_token.id).await?;

    // Mark user's email as verified
    let user = user_repository::mark_email_verified(pool, stored_token.user_id).await?;

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
    // Check rate limit: 3 requests per hour
    if !rate_limit_repository::check_and_increment(
        pool,
        email,
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
    let user = match user_repository::find_by_email(pool, email).await? {
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
    let token_hash = hash_token(&token)?;

    // Delete any existing password reset tokens for this user
    verification_token_repository::delete_user_tokens(
        pool,
        user.id,
        TokenType::PasswordReset,
    )
    .await?;

    // Create new token with 30-minute expiration
    let expires_at = Utc::now() + Duration::minutes(30);
    verification_token_repository::create(
        pool,
        user.id,
        &token_hash,
        TokenType::PasswordReset,
        expires_at,
    )
    .await?;

    // Generate reset link
    let reset_link = format!("{}/reset-password?token={}", config.frontend_url, token);

    // Send email
    email_service
        .send_password_reset_email(&user.email, &user.name, &reset_link)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_email = %user.email,
                "Failed to send password reset email"
            );
            ApiError::InternalServerError
        })?;

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
    let token_hash = hash_token(token_string)?;

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

    // Hash the new password
    let password_hash = hash(new_password, DEFAULT_COST)?;

    // Mark token as used
    verification_token_repository::mark_as_used(pool, stored_token.id).await?;

    // Update user's password
    let user = user_repository::update_password(pool, stored_token.user_id, &password_hash).await?;

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
