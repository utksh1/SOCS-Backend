use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::user::{SafeUser, UserRole},
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

// Optional auth middleware - tries to authenticate but doesn't fail if no token
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract token from Authorization header
    if let Some(auth_header) = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // Try to verify token and get user
            if let Ok(claims) = jwt::verify_token(token, &state.config.jwt_secret) {
                if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                    if let Ok(user) = auth_service::get_user_by_id(&state.db, user_id).await {
                        // Insert Some(user) into extensions
                        request.extensions_mut().insert(Some(user));
                        return next.run(request).await;
                    }
                }
            }
        }
    }
    
    // Insert None into extensions (no authenticated user)
    request.extensions_mut().insert(None::<crate::models::user::SafeUser>);
    next.run(request).await
}

// Middleware to check if user has TopLead role (replaces admin check)
pub async fn require_toplead(
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
    
    // Check if user has TopLead role
    if !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden("TopLead access required".to_string()));
    }
    
    // Insert user into request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}

// Authorization helper functions for use in route handlers

/// Check if user can manage (update/delete) a target user
pub fn can_manage_user(actor: &SafeUser, target: &SafeUser) -> Result<(), ApiError> {
    let actor_level = actor.role_level();
    let target_level = target.role_level();
    
    if actor_level <= target_level {
        return Err(ApiError::Forbidden(
            "You don't have permission to manage this user".to_string()
        ));
    }
    
    Ok(())
}

/// Check if user can assign a specific role
pub fn can_assign_role(actor: &SafeUser, role: &UserRole) -> Result<(), ApiError> {
    if actor.role_level() < role.level() {
        return Err(ApiError::Forbidden(
            format!("You cannot assign {:?} role (insufficient level)", role)
        ));
    }
    Ok(())
}

/// Check if user can assign all roles in a list
pub fn can_assign_roles(actor: &SafeUser, roles: &[UserRole]) -> Result<(), ApiError> {
    let actor_level = actor.role_level();
    
    for role in roles {
        if role.level() > actor_level {
            return Err(ApiError::Forbidden(
                format!("You cannot assign {:?} role (insufficient level)", role)
            ));
        }
    }
    
    Ok(())
}

/// Check if user can create/delete users
pub fn can_create_delete_users(user: &SafeUser) -> Result<(), ApiError> {
    if !user.highest_role().can_create_delete_users() {
        return Err(ApiError::Forbidden(
            "Only TopLead or Mentor can create/delete users".to_string()
        ));
    }
    Ok(())
}

/// Check if user can approve content (Mentor or TopLead)
pub fn can_approve_content(user: &SafeUser) -> Result<(), ApiError> {
    if !user.has_role(&UserRole::Mentor) && !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden(
            "Only Mentor or TopLead can approve content".to_string()
        ));
    }
    Ok(())
}

/// Check if user is the owner or has higher role level
pub fn is_owner_or_higher(actor: &SafeUser, owner_id: Uuid, min_role_level: u8) -> Result<(), ApiError> {
    if actor.id == owner_id {
        return Ok(());
    }
    
    if actor.role_level() >= min_role_level {
        return Ok(());
    }
    
    Err(ApiError::Forbidden(
        "You don't have permission to access this resource".to_string()
    ))
}
