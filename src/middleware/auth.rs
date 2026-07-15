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

// Middleware to check if user has TopLead tier (replaces admin check)
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
    
    // Check if user has TopLead tier in team table
    let user_tier: Option<crate::models::team::MemberTier> = sqlx::query_scalar(
        "SELECT tier FROM team WHERE created_by = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    
    if !matches!(user_tier, Some(crate::models::team::MemberTier::TopLead)) {
        return Err(ApiError::Forbidden("TopLead access required".to_string()));
    }
    
    // Insert user into request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}

// Permission check helpers - removed, now using tier-based checks
