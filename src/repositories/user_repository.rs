use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::User;

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (name, email, password, role)
        VALUES ($1, $2, $3, 'MEMBER')
        RETURNING 
            id, name, email, password, 
            role, profile_picture,
            created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT 
            id, name, email, password, 
            role, profile_picture,
            created_at, updated_at
        FROM users 
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT 
            id, name, email, password, 
            role, profile_picture,
            created_at, updated_at
        FROM users 
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_profile_picture(
    pool: &PgPool,
    user_id: Uuid,
    profile_picture: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET profile_picture = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(profile_picture)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
}
