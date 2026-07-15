use sqlx::PgPool;
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::{
    dto::auth_dto::{AuthResponse, LoginDto, RegisterDto},
    error::{ApiError, Result},
    models::user::SafeUser,
    repositories::user_repository,
    utils::jwt,
};

pub async fn register(
    pool: &PgPool,
    jwt_secret: &str,
    jwt_expires_in: i64,
    payload: RegisterDto,
) -> Result<AuthResponse> {
    // Check if email already exists
    if user_repository::find_by_email(pool, &payload.email)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict("Email already registered".to_string()));
    }
    
    // Hash password
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| ApiError::InternalServerError)?;
    
    // Create user with default Member role
    let user = user_repository::create(pool, &payload.name, &payload.email, &password_hash).await?;
    
    // Generate token
    let token = jwt::create_token(user.id, user.role.clone(), jwt_secret, jwt_expires_in)
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
    // Find user by email
    let user = user_repository::find_by_email(pool, &payload.email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;
    
    // Verify password
    let valid = verify(&payload.password, &user.password)
        .map_err(|_| ApiError::InternalServerError)?;
    
    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }
    
    // Generate token
    let token = jwt::create_token(user.id, user.role.clone(), jwt_secret, jwt_expires_in)
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
