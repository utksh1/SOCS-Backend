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
        self.roles = vec![role];
        self
    }
    
    pub fn roles(mut self, roles: Vec<UserRole>) -> Self {
        self.roles = roles;
        self
    }
    
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }
    
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }
    
    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }
    
    pub fn bio(mut self, bio: impl Into<String>) -> Self {
        self.bio = Some(bio.into());
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
                id, email, name, password, roles, slug, position, bio, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
            now,
            now
        )
        .fetch_one(pool)
        .await?;
        
        // Construct SafeUser manually
        let user = SafeUser {
            id: record.id,
            email: record.email,
            name: record.name,
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
