use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request to resend verification email (no body, uses authenticated user)
/// This is a marker struct for type safety
#[derive(Debug, Deserialize)]
pub struct ResendVerificationDto {}

/// Request to verify email with token
#[derive(Debug, Deserialize, Validate)]
pub struct VerifyEmailDto {
    #[validate(length(equal = 64))]
    pub token: String,
}

/// Request to initiate password reset
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordDto {
    #[validate(email)]
    pub email: String,
}

/// Request to reset password with token
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordDto {
    #[validate(length(equal = 64))]
    pub token: String,
    
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// Response for verification operations
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub success: bool,
    pub message: String,
}

impl VerificationResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}
