use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use crate::{
    dto::{
        auth_dto::{LoginDto, RegisterDto},
        verification_dto::{ForgotPasswordDto, ResetPasswordDto, VerificationResponse},
    },
    error::Result,
    models::user::SafeUser,
    services::{auth_service, email_service::EmailService, verification_service},
    repositories::user_repository,
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNameDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordDto {
    #[validate(length(min = 1))]
    pub current_password: String,
    
    #[validate(length(min = 8))]
    pub new_password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // Validate
    payload.validate()?;
    
    // Register user
    let result = auth_service::register(
        &state.db,
        &state.config.jwt_secret,
        state.config.jwt_expires_in,
        payload,
    )
    .await?;
    
    // Get the registered user's ID to fetch full user for email
    let user_id = result.user.id;
    
    // Fetch full user from database for email sending
    if let Ok(Some(full_user)) = user_repository::find_by_id(&state.db, user_id).await {
        // Create email service
        if let Ok(email_service) = EmailService::new() {
            // Send verification email (don't fail registration if email fails)
            if let Err(e) = verification_service::send_verification_email(
                &state.db,
                &state.config,
                &email_service,
                &full_user,
            )
            .await
            {
                tracing::error!(
                    error = ?e,
                    user_id = %user_id,
                    "Failed to send verification email during registration"
                );
                // Don't fail the registration, just log the error
            }
        } else {
            tracing::error!(
                user_id = %user_id,
                "Failed to initialize EmailService during registration"
            );
        }
    } else {
        tracing::error!(
            user_id = %user_id,
            "Failed to fetch user for verification email during registration"
        );
    }
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "User registered successfully. Please check your email to verify your account.",
            "data": result,
        })),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<serde_json::Value>> {
    // Validate
    payload.validate()?;
    
    // Login user
    let result = auth_service::login(
        &state.db,
        &state.config.jwt_secret,
        state.config.jwt_expires_in,
        payload,
    )
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Login successful",
        "data": result,
    })))
}

pub async fn get_me(Extension(user): Extension<SafeUser>) -> Result<Json<serde_json::Value>> {
    Ok(Json(json!({
        "success": true,
        "data": user
    })))
}

pub async fn update_name(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<UpdateNameDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Update name in database
    sqlx::query("UPDATE users SET name = $1, updated_at = NOW() WHERE id = $2")
        .bind(&payload.name)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Name updated successfully"
    })))
}

pub async fn change_password(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<ChangePasswordDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Get user from database to verify current password
    let db_user = user_repository::find_by_id(&state.db, user.id)
        .await?
        .ok_or_else(|| crate::error::ApiError::NotFound("User not found".to_string()))?;
    
    // Verify current password
    let valid = bcrypt::verify(&payload.current_password, &db_user.password)
        .map_err(|_| crate::error::ApiError::InternalServerError)?;
    
    if !valid {
        return Err(crate::error::ApiError::Unauthorized("Current password is incorrect".to_string()));
    }
    
    // Hash new password
    let new_password_hash = bcrypt::hash(&payload.new_password, bcrypt::DEFAULT_COST)
        .map_err(|_| crate::error::ApiError::InternalServerError)?;
    
    // Update password in database
    sqlx::query("UPDATE users SET password = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_password_hash)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Password changed successfully"
    })))
}

/// Resend verification email to authenticated user
pub async fn resend_verification(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<VerificationResponse>> {
    // Check if already verified
    if user.email_verified_at.is_some() {
        return Ok(Json(VerificationResponse::error("Email already verified")));
    }

    // Fetch full user from database
    let full_user = user_repository::find_by_id(&state.db, user.id)
        .await?
        .ok_or_else(|| crate::error::ApiError::NotFound("User not found".to_string()))?;

    // Create email service
    let email_service = EmailService::new()
        .map_err(|_| {
            tracing::error!("Failed to initialize EmailService");
            crate::error::ApiError::InternalServerError
        })?;

    // Send verification email
    verification_service::send_verification_email(
        &state.db,
        &state.config,
        &email_service,
        &full_user,
    )
    .await?;

    Ok(Json(VerificationResponse::success(
        "Verification email sent. Please check your inbox."
    )))
}

/// Verify email with token from query parameter
pub async fn verify_email(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<VerificationResponse>> {
    let token = params
        .get("token")
        .ok_or_else(|| crate::error::ApiError::BadRequest("Token parameter required".to_string()))?;

    // Validate token length
    if token.len() != 64 {
        return Err(crate::error::ApiError::BadRequest("Invalid token format".to_string()));
    }

    // Verify the token
    verification_service::verify_email_token(&state.db, token).await?;

    Ok(Json(VerificationResponse::success(
        "Email verified successfully!"
    )))
}

/// Request password reset email
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordDto>,
) -> Result<Json<VerificationResponse>> {
    payload.validate()?;

    // Create email service
    let email_service = EmailService::new()
        .map_err(|_| {
            tracing::error!("Failed to initialize EmailService");
            crate::error::ApiError::InternalServerError
        })?;

    // Send password reset email (always returns success to prevent email enumeration)
    verification_service::send_password_reset_email(
        &state.db,
        &state.config,
        &email_service,
        &payload.email,
    )
    .await?;

    Ok(Json(VerificationResponse::success(
        "If that email exists, you'll receive a password reset link shortly."
    )))
}

/// Reset password with token
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordDto>,
) -> Result<Json<VerificationResponse>> {
    payload.validate()?;

    // Create email service
    let email_service = EmailService::new()
        .map_err(|_| {
            tracing::error!("Failed to initialize EmailService");
            crate::error::ApiError::InternalServerError
        })?;

    // Reset the password
    verification_service::reset_password_with_token(
        &state.db,
        &email_service,
        &payload.token,
        &payload.new_password,
    )
    .await?;

    Ok(Json(VerificationResponse::success(
        "Password reset successfully. You can now log in with your new password."
    )))
}
