use sqlx::PgPool;
use uuid::Uuid;

use crate::models::team::{TeamMember, MemberTier};

pub async fn create(
    pool: &PgPool,
    slug: &str,
    name: &str,
    role: &str,
    bio: Option<&str>,
    skills: &[String],
    tier: MemberTier,
    github: Option<&str>,
    linkedin: Option<&str>,
    avatar_url: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<TeamMember, sqlx::Error> {
    sqlx::query_as::<_, TeamMember>(
        r#"
        INSERT INTO team (slug, name, role, bio, skills, tier, github, linkedin, avatar_url, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, slug, name, role, bio, skills, tier, github, linkedin, avatar_url, created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(name)
    .bind(role)
    .bind(bio)
    .bind(skills)
    .bind(tier)
    .bind(github)
    .bind(linkedin)
    .bind(avatar_url)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<TeamMember>, sqlx::Error> {
    sqlx::query_as::<_, TeamMember>(
        r#"
        SELECT id, slug, name, role, bio, skills, tier, github, linkedin, avatar_url, created_by, created_at, updated_at
        FROM team
        ORDER BY tier, created_at
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TeamMember>, sqlx::Error> {
    sqlx::query_as::<_, TeamMember>(
        r#"
        SELECT id, slug, name, role, bio, skills, tier, github, linkedin, avatar_url, created_by, created_at, updated_at
        FROM team
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<TeamMember>, sqlx::Error> {
    sqlx::query_as::<_, TeamMember>(
        r#"
        SELECT id, slug, name, role, bio, skills, tier, github, linkedin, avatar_url, created_by, created_at, updated_at
        FROM team
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM team WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
