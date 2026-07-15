use sqlx::PgPool;
use uuid::Uuid;
use crate::models::blog_post::BlogPost;



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
