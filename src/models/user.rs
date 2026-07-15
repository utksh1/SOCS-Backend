use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    TopLead,
    Mentor,
    Core,
    Lead,
    Member,
}

impl UserRole {
    /// Returns the hierarchical level (higher number = higher authority)
    pub fn level(&self) -> u8 {
        match self {
            UserRole::TopLead => 5,
            UserRole::Mentor => 4,
            UserRole::Core => 3,
            UserRole::Lead => 2,
            UserRole::Member => 1,
        }
    }
    
    /// Check if this role can manage another role
    pub fn can_manage(&self, other: &UserRole) -> bool {
        self.level() >= other.level()
    }
    
    /// Check if this role can create/delete users
    pub fn can_create_delete_users(&self) -> bool {
        matches!(self, UserRole::TopLead | UserRole::Mentor)
    }
    
    /// Check if this role has management capabilities
    pub fn can_manage_users(&self) -> bool {
        matches!(self, UserRole::TopLead | UserRole::Mentor | UserRole::Core)
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub role: UserRole,
    pub slug: Option<String>,
    pub position: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub github: Option<String>,
    pub linkedin: Option<String>,
    pub avatar_url: Option<String>,
    pub profile_picture: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SafeUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub slug: Option<String>,
    pub position: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub github: Option<String>,
    pub linkedin: Option<String>,
    pub avatar_url: Option<String>,
    pub profile_picture: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for SafeUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            role: user.role,
            slug: user.slug,
            position: user.position,
            bio: user.bio,
            skills: user.skills,
            github: user.github,
            linkedin: user.linkedin,
            avatar_url: user.avatar_url,
            profile_picture: user.profile_picture,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}
