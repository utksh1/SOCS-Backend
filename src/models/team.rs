use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq, Clone)]
#[sqlx(type_name = "member_tier", rename_all = "lowercase")]
pub enum MemberTier {
    TopLead,
    Mentor,
    Core,
    Lead,
    Member,
}

impl MemberTier {
    /// Returns the hierarchical level (higher number = higher authority)
    pub fn level(&self) -> u8 {
        match self {
            MemberTier::TopLead => 5,
            MemberTier::Mentor => 4,
            MemberTier::Core => 3,
            MemberTier::Lead => 2,
            MemberTier::Member => 1,
        }
    }
    
    /// Check if this tier can manage another tier
    pub fn can_manage(&self, other: &MemberTier) -> bool {
        self.level() >= other.level()
    }
    
    /// Check if this tier can manage team members
    pub fn can_manage_team(&self) -> bool {
        matches!(self, MemberTier::TopLead | MemberTier::Mentor | MemberTier::Core)
    }
    
    /// Check if this tier can create/delete members
    pub fn can_create_delete_members(&self) -> bool {
        matches!(self, MemberTier::TopLead | MemberTier::Mentor)
    }
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
