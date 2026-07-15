use axum::{
    extract::State,
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::{
    dto::auth_dto::{LoginDto, RegisterDto},
    error::Result,
    models::user::SafeUser,
    services::auth_service,
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
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "User registered successfully",
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
