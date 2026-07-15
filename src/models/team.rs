use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "member_tier", rename_all = "lowercase")]
pub enum MemberTier {
    Core,
    Lead,
    Member,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMember {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub role: String,
    pub bio: Option<String>,
    pub skills: Vec<String>,
    pub tier: MemberTier,
    pub github: Option<String>,
    pub linkedin: Option<String>,
    pub avatar_url: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
