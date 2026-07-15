use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::user::UserRole,
    services::auth_service,
    utils::jwt,
    AppState,
};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;
    
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Invalid authorization header".to_string()))?;
    
    // Verify token
    let claims = jwt::verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid token".to_string()))?;
    
    // Get user from database
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;
    
    let user = auth_service::get_user_by_id(&state.db, user_id).await?;
    
    // Insert user into request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}

// Middleware to check if user is admin
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;
    
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Invalid authorization header".to_string()))?;
    
    // Verify token
    let claims = jwt::verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid token".to_string()))?;
    
    // Get user from database
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;
    
    let user = auth_service::get_user_by_id(&state.db, user_id).await?;
    
    // Check if user has admin role
    if user.role != UserRole::Admin {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }
    
    // Insert user into request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}

// Permission check helpers
pub fn check_event_permission(user_role: &UserRole) -> Result<(), ApiError> {
    if !user_role.can_manage_events() {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage events".to_string(),
        ));
    }
    Ok(())
}

pub fn check_blog_permission(user_role: &UserRole) -> Result<(), ApiError> {
    if !user_role.can_manage_blog() {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage blog posts".to_string(),
        ));
    }
    Ok(())
}

pub fn check_resource_permission(user_role: &UserRole) -> Result<(), ApiError> {
    if !user_role.can_manage_resources() {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage resources".to_string(),
        ));
    }
    Ok(())
}

pub fn check_team_permission(user_role: &UserRole) -> Result<(), ApiError> {
    if !user_role.can_manage_team() {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage team members".to_string(),
        ));
    }
    Ok(())
}

pub fn check_user_management_permission(user_role: &UserRole) -> Result<(), ApiError> {
    if !user_role.can_manage_users() {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage users".to_string(),
        ));
    }
    Ok(())
}
