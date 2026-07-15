use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum TokenType {
    #[sqlx(rename = "email_verification")]
    EmailVerification,
    #[sqlx(rename = "password_reset")]
    PasswordReset,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::EmailVerification => "email_verification",
            TokenType::PasswordReset => "password_reset",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    #[sqlx(try_from = "String")]
    pub token_type: TokenType,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl VerificationToken {
    /// Check if token is still valid (not expired and not used)
    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && self.expires_at > Utc::now()
    }
    
    /// Check if token has been used
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }
    
    /// Check if token has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

// Implement conversion from String for sqlx
impl TryFrom<String> for TokenType {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "email_verification" => Ok(TokenType::EmailVerification),
            "password_reset" => Ok(TokenType::PasswordReset),
            _ => Err(format!("Invalid token type: {}", s)),
        }
    }
}
