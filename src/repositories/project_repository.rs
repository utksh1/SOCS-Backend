use sqlx::PgPool;
use uuid::Uuid;

use crate::models::project::Project;

pub async fn create(
    pool: &PgPool,
    slug: &str,
    title: &str,
    description: &str,
    tech_stack: &[String],
    github_link: Option<&str>,
    tags: &[String],
    featured: bool,
    created_by: Option<Uuid>,
) -> Result<Project, sqlx::Error> {
    let project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, github_link, tags, featured, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, slug, title, description, tech_stack, github_link, tags, featured, created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(title)
    .bind(description)
    .bind(tech_stack)
    .bind(github_link)
    .bind(tags)
    .bind(featured)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    
    Ok(project)
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Project>, sqlx::Error> {
    let projects = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, slug, title, description, tech_stack, github_link, tags, featured, created_by, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    
    Ok(projects)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Project>, sqlx::Error> {
    let project = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, slug, title, description, tech_stack, github_link, tags, featured, created_by, created_at, updated_at
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(project)
}

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Project>, sqlx::Error> {
    let project = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, slug, title, description, tech_stack, github_link, tags, featured, created_by, created_at, updated_at
        FROM projects
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    
    Ok(project)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    slug: Option<&str>,
    title: Option<&str>,
    description: Option<&str>,
    tech_stack: Option<&[String]>,
    github_link: Option<Option<&str>>,
    tags: Option<&[String]>,
    featured: Option<bool>,
) -> Result<Project, sqlx::Error> {
    use sqlx::QueryBuilder;
    
    let mut builder = QueryBuilder::new("UPDATE projects SET updated_at = NOW()");
    let mut has_updates = false;
    
    if let Some(s) = slug {
        builder.push(", slug = ").push_bind(s);
        has_updates = true;
    }
    if let Some(t) = title {
        builder.push(", title = ").push_bind(t);
        has_updates = true;
    }
    if let Some(d) = description {
        builder.push(", description = ").push_bind(d);
        has_updates = true;
    }
    if let Some(ts) = tech_stack {
        builder.push(", tech_stack = ").push_bind(ts);
        has_updates = true;
    }
    if let Some(gh) = github_link {
        builder.push(", github_link = ").push_bind(gh);
        has_updates = true;
    }
    if let Some(tg) = tags {
        builder.push(", tags = ").push_bind(tg);
        has_updates = true;
    }
    if let Some(f) = featured {
        builder.push(", featured = ").push_bind(f);
        has_updates = true;
    }
    
    builder.push(" WHERE id = ").push_bind(id);
    builder.push(" RETURNING *");
    
    let project = builder.build_query_as::<Project>()
        .fetch_one(pool)
        .await?;
    
    Ok(project)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
