use sqlx::PgPool;
use uuid::Uuid;
use crate::models::blog_post::BlogPost;



pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, approval_status, approved_by, approved_at, tags, featured_image, author_id, published_at, created_at, updated_at, deleted_at
        FROM blog_posts
        WHERE slug = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE blog_posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE blog_posts SET deleted_at = NULL WHERE id = $1"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn permanent_delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM blog_posts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
