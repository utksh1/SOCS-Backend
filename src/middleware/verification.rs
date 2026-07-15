use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
    Extension,
};

use crate::{error::ApiError, models::user::SafeUser};

/// Middleware to require email verification for critical actions
pub async fn require_verified_email(
    Extension(user): Extension<SafeUser>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if user.email_verified_at.is_none() {
        tracing::warn!(
            user_id = %user.id,
            user_email = %user.email,
            path = %request.uri().path(),
            "Unverified user attempted to access protected resource"
        );
        
        return Err(ApiError::Forbidden(
            "Email verification required for this action. Please check your email for the verification link.".to_string()
        ));
    }

    Ok(next.run(request).await)
}
