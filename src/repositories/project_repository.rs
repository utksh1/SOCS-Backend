use sqlx::PgPool;
use uuid::Uuid;

use crate::models::project::Project;



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

#[allow(clippy::too_many_arguments)]
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
    let mut _has_updates = false;
    
    if let Some(s) = slug {
        builder.push(", slug = ").push_bind(s);
        _has_updates = true;
    }
    if let Some(t) = title {
        builder.push(", title = ").push_bind(t);
        _has_updates = true;
    }
    if let Some(d) = description {
        builder.push(", description = ").push_bind(d);
        _has_updates = true;
    }
    if let Some(ts) = tech_stack {
        builder.push(", tech_stack = ").push_bind(ts);
        _has_updates = true;
    }
    if let Some(gh) = github_link {
        builder.push(", github_link = ").push_bind(gh);
        _has_updates = true;
    }
    if let Some(tg) = tags {
        builder.push(", tags = ").push_bind(tg);
        _has_updates = true;
    }
    if let Some(f) = featured {
        builder.push(", featured = ").push_bind(f);
        _has_updates = true;
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
