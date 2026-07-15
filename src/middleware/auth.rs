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
    if user.role != UserRole::TopLead {
        return Err(ApiError::Forbidden("TopLead access required".to_string()));
    }
    
    // Insert user into request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}
