use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct AuthFixture;

impl AuthFixture {
    /// Generate JWT token for a user ID
    pub fn generate_token(user_id: Uuid, secret: &str) -> String {
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(1))
            .unwrap()
            .timestamp() as usize;
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
        };
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }
    
    /// Generate expired token for testing
    pub fn generate_expired_token(user_id: Uuid, secret: &str) -> String {
        let expiration = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::hours(1))
            .unwrap()
            .timestamp() as usize;
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
        };
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }
    
    /// Generate token with invalid signature
    pub fn generate_invalid_token(user_id: Uuid) -> String {
        Self::generate_token(user_id, "wrong_secret_key_that_wont_verify")
    }
}
