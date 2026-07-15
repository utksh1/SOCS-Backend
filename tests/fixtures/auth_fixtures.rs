use socs_backend::{models::user::UserRole, utils::jwt::create_token};
use uuid::Uuid;

pub struct AuthFixture;

impl AuthFixture {
    /// Generate JWT token for a user ID
    pub fn generate_token(user_id: Uuid, secret: &str) -> String {
        Self::generate_token_with_role(user_id, UserRole::Member, secret)
    }

    pub fn generate_token_with_role(user_id: Uuid, role: UserRole, secret: &str) -> String {
        create_token(user_id, role, 0, secret, 3600).unwrap()
    }
    
    /// Generate expired token for testing
    pub fn generate_expired_token(user_id: Uuid, secret: &str) -> String {
        create_token(user_id, UserRole::Member, 0, secret, -3600).unwrap()
    }
    
    /// Generate token with invalid signature
    pub fn generate_invalid_token(user_id: Uuid) -> String {
        Self::generate_token(user_id, "wrong_secret_key_that_wont_verify")
    }
}
