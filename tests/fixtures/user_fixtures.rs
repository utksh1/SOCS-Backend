use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use bcrypt::{hash, DEFAULT_COST};

use socs_backend::models::user::{SafeUser, UserRole};

pub struct UserFixture {
    pub email: String,
    pub name: String,
    pub roles: Vec<UserRole>,
    pub password: String,
    pub slug: Option<String>,
    pub position: Option<String>,
    pub bio: Option<String>,
}

impl UserFixture {
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        Self {
            email: format!("user_{}@test.com", id),
            name: format!("Test User {}", &id.to_string()[..8]),
            roles: vec![UserRole::Member],
            password: "Test123!@#".to_string(),
            slug: None,
            position: None,
            bio: None,
        }
    }
    
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }
    
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    pub fn role(mut self, role: UserRole) -> Self {
        if role != UserRole::Member {
            self.roles = vec![role, UserRole::Member];
        } else {
            self.roles = vec![role];
        }
        self
    }
    
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }
    
    /// Insert user into database and return SafeUser
    pub async fn insert(self, pool: &PgPool) -> Result<SafeUser, sqlx::Error> {
        let hashed_password = hash(&self.password, DEFAULT_COST).unwrap();
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        // Insert and get basic user data
        let record = sqlx::query!(
            r#"
            INSERT INTO users (
                id, email, name, password, roles, slug, position, bio, email_verified_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING 
                id, email, name, 
                roles as "roles: Vec<UserRole>",
                email_verified_at,
                slug, position, bio,
                skills, github, linkedin, avatar_url, profile_picture,
                created_at, updated_at
            "#,
            id,
            self.email,
            self.name,
            hashed_password,
            &self.roles as &[UserRole],
            self.slug,
            self.position,
            self.bio,
            now, // email_verified_at
            now, // created_at
            now  // updated_at
        )
        .fetch_one(pool)
        .await?;
        
        // Construct SafeUser manually
        let user = SafeUser {
            id: record.id,
            email: record.email,
            name: record.name,
            token_version: 0,
            roles: record.roles,
            role: None, // Computed field
            email_verified_at: record.email_verified_at,
            slug: record.slug,
            position: record.position,
            bio: record.bio,
            skills: record.skills,
            github: record.github,
            linkedin: record.linkedin,
            avatar_url: record.avatar_url,
            profile_picture: record.profile_picture,
            created_at: record.created_at,
            updated_at: record.updated_at,
        };
        
        Ok(user)
    }
}

impl Default for UserFixture {
    fn default() -> Self {
        Self::new()
    }
}
