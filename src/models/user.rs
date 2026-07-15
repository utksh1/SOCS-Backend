use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "UPPERCASE")]
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
    
    /// Check if this role can create/delete users
    pub fn can_create_delete_users(&self) -> bool {
        matches!(self, UserRole::TopLead | UserRole::Mentor)
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing)]
    #[sqlx(default)]
    pub token_version: i32,
    pub roles: Vec<UserRole>,  // Changed from single role to multiple roles
    pub email_verified_at: Option<DateTime<Utc>>,
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

impl User {
    /// Get the highest role (for display purposes)
    pub fn highest_role(&self) -> &UserRole {
        self.roles.iter()
            .max_by_key(|r| r.level())
            .unwrap_or(&UserRole::Member)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SafeUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    #[sqlx(default)]
    pub token_version: i32,
    pub roles: Vec<UserRole>,  // Changed from single role to multiple roles
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub role: Option<UserRole>,  // Computed field: highest role for display
    pub email_verified_at: Option<DateTime<Utc>>,
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

impl SafeUser {
    /// Get the highest role (for display purposes)
    pub fn highest_role(&self) -> &UserRole {
        self.roles.iter()
            .max_by_key(|r| r.level())
            .unwrap_or(&UserRole::Member)
    }
    
    /// Check if user has a specific role
    pub fn has_role(&self, role: &UserRole) -> bool {
        self.roles.contains(role)
    }
    
    /// Get role level (highest role's level)
    pub fn role_level(&self) -> u8 {
        self.highest_role().level()
    }
}

impl From<User> for SafeUser {
    fn from(user: User) -> Self {
        let highest = user.highest_role().clone();
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            token_version: user.token_version,
            roles: user.roles,
            role: Some(highest),  // Set the highest role for display
            email_verified_at: user.email_verified_at,
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

/// User data that is safe to expose from public directory endpoints.
/// Authentication and administration endpoints should continue to use
/// `SafeUser`, which deliberately includes the account email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub name: String,
    pub roles: Vec<UserRole>,
    pub role: Option<UserRole>,
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

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        let highest = user.highest_role().clone();
        Self {
            id: user.id,
            name: user.name,
            roles: user.roles,
            role: Some(highest),
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

impl From<SafeUser> for PublicUser {
    fn from(user: SafeUser) -> Self {
        Self {
            id: user.id,
            name: user.name,
            roles: user.roles,
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
