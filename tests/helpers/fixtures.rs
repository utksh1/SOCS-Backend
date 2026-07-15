use socs_backend::models::user::{SafeUser, UserRole};
use socs_backend::models::event::{Event, EventType, EventStatus};
use socs_backend::utils::jwt::create_token;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};

pub struct UserFixture;

impl UserFixture {
    pub async fn create(pool: &PgPool, name: &str, email: &str, roles: Vec<UserRole>) -> (SafeUser, String) {
        let unique_suffix = uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>();
        let slug = format!("{}-{}", socs_backend::utils::slugify::slugify(name), unique_suffix);
        let unique_email = format!("{}_{}", unique_suffix, email);
        
        let user = sqlx::query_as::<_, socs_backend::models::user::User>(
            r#"
            INSERT INTO users (name, email, password, roles, slug, email_verified_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, name, email, password, roles, email_verified_at, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at
            "#
        )
        .bind(name)
        .bind(unique_email)
        .bind("hashed_password_for_tests")
        .bind(&roles)
        .bind(&slug)
        .fetch_one(pool)
        .await
        .expect("Failed to create user fixture");

        let safe_user = SafeUser::from(user);
        
        // Generate JWT token
        // Use the same mock secret as in TestApp config
        // Pass the first role as the main role for token creation
        let token = create_token(safe_user.id, roles.first().unwrap().clone(), 0, "test_secret_key_12345", 3600)
            .expect("Failed to generate token");
            
        (safe_user, token)
    }

    pub async fn create_toplead(pool: &PgPool) -> (SafeUser, String) {
        Self::create(pool, "TopLead User", "toplead@test.com", vec![UserRole::TopLead, UserRole::Member]).await
    }

    pub async fn create_mentor(pool: &PgPool) -> (SafeUser, String) {
        Self::create(pool, "Mentor User", "mentor@test.com", vec![UserRole::Mentor, UserRole::Member]).await
    }
    
    pub async fn create_core(pool: &PgPool) -> (SafeUser, String) {
        Self::create(pool, "Core User", "core@test.com", vec![UserRole::Core, UserRole::Member]).await
    }
    
    pub async fn create_lead(pool: &PgPool) -> (SafeUser, String) {
        Self::create(pool, "Lead User", "lead@test.com", vec![UserRole::Lead, UserRole::Member]).await
    }

    pub async fn create_member(pool: &PgPool) -> (SafeUser, String) {
        Self::create(pool, "Member User", "member@test.com", vec![UserRole::Member]).await
    }
}

pub struct EventFixture;

impl EventFixture {
    pub async fn create(
        pool: &PgPool,
        title: &str,
        event_type: EventType,
        status: EventStatus,
        organizer_id: Option<Uuid>
    ) -> Event {
        let slug = socs_backend::utils::slugify::slugify(title);
        let date = Utc::now() + Duration::days(5);
        
        sqlx::query_as::<_, Event>(
            r#"
            INSERT INTO events (slug, title, description, date, type, status, location, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, slug, title, description, date, type, status, location, created_by, created_at, updated_at, deleted_at
            "#
        )
        .bind(&slug)
        .bind(title)
        .bind("Test event description")
        .bind(date)
        .bind(event_type)
        .bind(status)
        .bind("Test Location")
        .bind(organizer_id)
        .fetch_one(pool)
        .await
        .expect("Failed to create event fixture")
    }
}
