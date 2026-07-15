use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Member,
    EventOrganizer,
    BlogEditor,
    ResourceManager,
    TeamLead,
    Management,
    Admin,
}

impl UserRole {
    /// Check if this role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Check if this role has management privileges (management or admin)
    pub fn is_management(&self) -> bool {
        matches!(self, UserRole::Management | UserRole::Admin)
    }

    /// Check if this role can manage events
    pub fn can_manage_events(&self) -> bool {
        matches!(
            self,
            UserRole::EventOrganizer | UserRole::Management | UserRole::Admin
        )
    }

    /// Check if this role can manage blog posts
    pub fn can_manage_blog(&self) -> bool {
        matches!(
            self,
            UserRole::BlogEditor | UserRole::Management | UserRole::Admin
        )
    }

    /// Check if this role can manage resources
    pub fn can_manage_resources(&self) -> bool {
        matches!(
            self,
            UserRole::ResourceManager | UserRole::Management | UserRole::Admin
        )
    }

    /// Check if this role can manage team members
    pub fn can_manage_team(&self) -> bool {
        matches!(
            self,
            UserRole::TeamLead | UserRole::Management | UserRole::Admin
        )
    }

    /// Check if this role can manage users
    pub fn can_manage_users(&self) -> bool {
        self.is_admin()
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
            profile_picture: user.profile_picture,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}
