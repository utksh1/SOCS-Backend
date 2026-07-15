use sqlx::PgPool;
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::{
    dto::auth_dto::{AuthResponse, LoginDto, RegisterDto},
    error::{ApiError, Result},
    models::user::SafeUser,
    repositories::user_repository,
    utils::sanitize::{normalize_email, sanitize_plain_text},
    utils::jwt,
};

pub async fn register(
    pool: &PgPool,
    jwt_secret: &str,
    jwt_expires_in: i64,
    payload: RegisterDto,
) -> Result<AuthResponse> {
    let email = normalize_email(&payload.email);

    // Check if email already exists
    if user_repository::find_by_email(pool, &email)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict("Email already registered".to_string()));
    }
    
    // Hash password in a blocking task
    let password_clone = payload.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || {
        hash(&password_clone, DEFAULT_COST)
    })
    .await
    .map_err(|_| ApiError::InternalServerError)?
    .map_err(|_| ApiError::InternalServerError)?;
    
    // Create user with default Member role
    let name = sanitize_plain_text(&payload.name);
    let user = user_repository::create(pool, &name, &email, &password_hash).await?;
    
    // Generate token with highest role and token_version
    let highest_role = user.highest_role().clone();
    let token = jwt::create_token(user.id, highest_role, user.token_version, jwt_secret, jwt_expires_in)
        .map_err(|_| ApiError::InternalServerError)?;
    
    Ok(AuthResponse {
        token,
        user: user.into(),
    })
}

pub async fn login(
    pool: &PgPool,
    jwt_secret: &str,
    jwt_expires_in: i64,
    payload: LoginDto,
) -> Result<AuthResponse> {
    let email = normalize_email(&payload.email);

    // Find user by email
    let user = user_repository::find_by_email(pool, &email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;
    
    // Verify password in a blocking task
    let password_clone = payload.password.clone();
    let user_password_clone = user.password.clone();
    let valid = tokio::task::spawn_blocking(move || {
        verify(&password_clone, &user_password_clone)
    })
    .await
    .map_err(|_| ApiError::InternalServerError)?
    .map_err(|_| ApiError::InternalServerError)?;
    
    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }
    
    // Generate token with highest role and token_version
    let highest_role = user.highest_role().clone();
    let token = jwt::create_token(user.id, highest_role, user.token_version, jwt_secret, jwt_expires_in)
        .map_err(|_| ApiError::InternalServerError)?;
    
    Ok(AuthResponse {
        token,
        user: user.into(),
    })
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<SafeUser> {
    let user = user_repository::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
    
    Ok(user.into())
}
