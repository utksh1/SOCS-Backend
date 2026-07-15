use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::blog_post::{BlogPost, PostCategory, PostStatus};

pub async fn create(
    pool: &PgPool,
    slug: &str,
    title: &str,
    excerpt: &str,
    content: &str,
    category: PostCategory,
    status: PostStatus,
    tags: &[String],
    featured_image: Option<&str>,
    author_id: Option<Uuid>,
    published_at: Option<DateTime<Utc>>,
) -> Result<BlogPost, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        INSERT INTO blog_posts (slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(title)
    .bind(excerpt)
    .bind(content)
    .bind(category)
    .bind(status)
    .bind(tags)
    .bind(featured_image)
    .bind(author_id)
    .bind(published_at)
    .fetch_one(pool)
    .await
}

pub async fn find_all_published(pool: &PgPool) -> Result<Vec<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        WHERE status = 'PUBLISHED'
        ORDER BY published_at DESC NULLS LAST, created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags, featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM blog_posts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
