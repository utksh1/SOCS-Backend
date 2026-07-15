use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use chrono::Utc;
use crate::{
    dto::{
        blog_post_dto::CreateBlogPostDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result,
    models::user::SafeUser,
    repositories::blog_post_repository,
    utils::slugify::slugify,
    AppState
};

// GET /api/blog - List published posts (public, with pagination)
pub async fn list_blog_posts(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM blog_posts WHERE status = 'PUBLISHED'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get paginated posts
    let posts: Vec<crate::models::blog_post::BlogPost> = sqlx::query_as(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags,
               featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        WHERE status = 'PUBLISHED'
        ORDER BY published_at DESC
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(params.limit)
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;
    
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(Json(json!({
        "success": true,
        "data": posts,
        "pagination": pagination
    })))
}

// GET /api/blog/all - List all posts including drafts (admin, with pagination)
pub async fn list_all_blog_posts(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blog_posts")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated posts
    let posts: Vec<crate::models::blog_post::BlogPost> = sqlx::query_as(
        r#"
        SELECT id, slug, title, excerpt, content, category, status, tags,
               featured_image, author_id, published_at, created_at, updated_at
        FROM blog_posts
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(params.limit)
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;
    
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(Json(json!({
        "success": true,
        "data": posts,
        "pagination": pagination
    })))
}

// GET /api/blog/slug/:slug - Get single post by slug (public for published, admin for drafts)
pub async fn get_blog_post(State(state): State<AppState>, Path(slug): Path<String>) -> Result<Json<serde_json::Value>> {
    let post = blog_post_repository::find_by_slug(&state.db, &slug).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Only allow published posts to be viewed publicly
    if post.status != crate::models::blog_post::PostStatus::Published {
        return Err(crate::error::ApiError::NotFound("Blog post not found".to_string()));
    }
    
    Ok(Json(json!({"success": true, "data": post})))
}

// POST /api/blog - Create blog post (requires blog editor permission)
pub async fn create_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateBlogPostDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // Check permission
    crate::middleware::auth::check_blog_permission(&user.role)?;
    
    payload.validate()?;
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.title));
    
    // Check if slug exists
    if blog_post_repository::find_by_slug(&state.db, &slug).await?.is_some() {
        return Err(crate::error::ApiError::Conflict("Blog post slug already exists".to_string()));
    }
    
    let status = payload.status.unwrap_or(crate::models::blog_post::PostStatus::Draft);
    let published_at = if status == crate::models::blog_post::PostStatus::Published {
        Some(payload.published_at.unwrap_or_else(|| Utc::now()))
    } else {
        None
    };
    
    let post = blog_post_repository::create(
        &state.db,
        &slug,
        &payload.title,
        &payload.excerpt,
        &payload.content,
        payload.category,
        status,
        &payload.tags,
        payload.featured_image.as_deref(),
        Some(user.id),
        published_at,
    ).await?;
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Blog post created", "data": post}))))
}

// DELETE /api/blog/id/:id - Delete blog post (requires blog editor permission)
pub async fn delete_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check permission
    crate::middleware::auth::check_blog_permission(&user.role)?;
    
    let deleted = blog_post_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Blog post not found".to_string()));
    }
    Ok(Json(json!({"success": true, "message": "Blog post deleted"})))
}
