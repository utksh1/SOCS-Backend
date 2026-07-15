use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use chrono::Utc;
use crate::{
    dto::{
        blog_post_dto::CreateBlogPostDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::{ApiError, Result},
    models::{
        user::{SafeUser, UserRole},
        blog_post::{BlogPost, PostStatus, ContentStatus, BlogCollaborator},
    },
    repositories::blog_post_repository,
    utils::slugify::slugify,
    AppState
};

#[derive(Debug, Deserialize)]
pub struct ApprovalDto {
    pub status: String, // "approved" or "rejected"
}

// GET /api/blog - List published and approved posts (public, with pagination)
pub async fn list_blog_posts(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count of published and approved posts
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM blog_posts WHERE status = 'PUBLISHED' AND approval_status = 'approved'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get paginated posts
    let posts: Vec<BlogPost> = sqlx::query_as(
        r#"
        SELECT * FROM blog_posts
        WHERE status = 'PUBLISHED' AND approval_status = 'approved'
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

// GET /api/blog/all - List all posts including pending (TopLead only)
pub async fn list_all_blog_posts(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    // Only TopLead can see all posts
    if !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden("Only TopLead can view all blog posts".to_string()));
    }
    
    params.validate();
    
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blog_posts")
        .fetch_one(&state.db)
        .await?;
    
    let posts: Vec<BlogPost> = sqlx::query_as(
        "SELECT * FROM blog_posts ORDER BY created_at DESC LIMIT $1 OFFSET $2"
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

// GET /api/blog/my - List user's own blog posts
pub async fn list_my_blog_posts(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    let posts: Vec<BlogPost> = sqlx::query_as(
        "SELECT * FROM blog_posts WHERE author_id = $1 ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": posts,
    })))
}

// GET /api/blog/slug/:slug - Get single post by slug (public for approved, or if user is author/collaborator)
pub async fn get_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<Option<SafeUser>>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let post: BlogPost = blog_post_repository::find_by_slug(&state.db, &slug).await?
        .ok_or_else(|| ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Check if user can view this post
    let can_view = match &user {
        Some(u) => {
            // TopLead can see everything
            u.has_role(&UserRole::TopLead) ||
            // Author can see their own
            post.author_id == Some(u.id) ||
            // Post is published and approved (public)
            (post.status == PostStatus::Published && post.approval_status == ContentStatus::Approved) ||
            // Check if user is a collaborator
            is_collaborator(&state.db, post.id, u.id).await?
        },
        None => post.status == PostStatus::Published && post.approval_status == ContentStatus::Approved,
    };
    
    if !can_view {
        return Err(ApiError::NotFound("Blog post not found".to_string()));
    }
    
    // Get collaborators
    let collaborators: Vec<BlogCollaborator> = sqlx::query_as(
        "SELECT * FROM blog_collaborators WHERE blog_post_id = $1"
    )
    .bind(post.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "post": post,
            "collaborators": collaborators,
        }
    })))
}

// POST /api/blog - Create blog post (any authenticated user, starts as pending)
pub async fn create_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateBlogPostDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.title));
    
    // Check if slug exists
    if blog_post_repository::find_by_slug(&state.db, &slug).await?.is_some() {
        return Err(ApiError::Conflict("Blog post slug already exists".to_string()));
    }
    
    let status = payload.status.unwrap_or(PostStatus::Draft);
    let published_at = if status == PostStatus::Published {
        Some(payload.published_at.unwrap_or_else(|| Utc::now()))
    } else {
        None
    };
    
    // Create with pending approval status (TopLead can create approved directly)
    let approval_status = if user.has_role(&UserRole::TopLead) {
        ContentStatus::Approved
    } else {
        ContentStatus::Pending
    };
    
    let post: BlogPost = sqlx::query_as(
        r#"
        INSERT INTO blog_posts (slug, title, excerpt, content, category, status, approval_status, tags, featured_image, author_id, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#
    )
    .bind(&slug)
    .bind(&payload.title)
    .bind(&payload.excerpt)
    .bind(&payload.content)
    .bind(payload.category)
    .bind(status)
    .bind(approval_status)
    .bind(&payload.tags)
    .bind(&payload.featured_image)
    .bind(user.id)
    .bind(published_at)
    .fetch_one(&state.db)
    .await?;
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": if post.approval_status == ContentStatus::Approved {
                "Blog post created and approved"
            } else {
                "Blog post submitted for approval"
            },
            "data": post
        }))
    ))
}

// PUT /api/blog/:id - Update blog post (only author, collaborators, or TopLead)
pub async fn update_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateBlogPostDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    let post: BlogPost = sqlx::query_as("SELECT * FROM blog_posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Check permissions: TopLead, author, or collaborator
    let is_author = post.author_id == Some(user.id);
    let is_collab = is_collaborator(&state.db, id, user.id).await?;
    let can_edit = user.has_role(&UserRole::TopLead) || is_author || is_collab;
    
    if !can_edit {
        return Err(ApiError::Forbidden("You don't have permission to edit this blog post".to_string()));
    }
    
    // Update the post
    let updated_post: BlogPost = sqlx::query_as(
        r#"
        UPDATE blog_posts
        SET slug = COALESCE($1, slug),
            title = COALESCE($2, title),
            excerpt = COALESCE($3, excerpt),
            content = COALESCE($4, content),
            category = COALESCE($5, category),
            status = COALESCE($6, status),
            tags = COALESCE($7, tags),
            featured_image = COALESCE($8, featured_image),
            updated_at = NOW()
        WHERE id = $9
        RETURNING *
        "#
    )
    .bind(payload.slug)
    .bind(payload.title)
    .bind(payload.excerpt)
    .bind(payload.content)
    .bind(payload.category)
    .bind(payload.status)
    .bind(payload.tags)
    .bind(payload.featured_image)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Blog post updated successfully",
        "data": updated_post,
    })))
}

// DELETE /api/blog/id/:id - Delete blog post (only author or TopLead)
pub async fn delete_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let post: BlogPost = sqlx::query_as("SELECT * FROM blog_posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Check permissions: TopLead or author
    let is_author = post.author_id == Some(user.id);
    let can_delete = user.has_role(&UserRole::TopLead) || is_author;
    
    if !can_delete {
        return Err(ApiError::Forbidden("You don't have permission to delete this blog post".to_string()));
    }
    
    let deleted = blog_post_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Blog post not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "Blog post deleted"
    })))
}

// POST /api/blog/:id/approve - Approve or reject blog post (TopLead only)
pub async fn approve_or_reject_blog_post(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ApprovalDto>,
) -> Result<Json<serde_json::Value>> {
    // Only TopLead can approve/reject
    if !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden("Only TopLead can approve or reject blog posts".to_string()));
    }
    
    let status = match payload.status.as_str() {
        "approved" => ContentStatus::Approved,
        "rejected" => ContentStatus::Rejected,
        _ => return Err(ApiError::BadRequest("Invalid status. Use 'approved' or 'rejected'".to_string())),
    };
    
    let post: BlogPost = sqlx::query_as(
        r#"
        UPDATE blog_posts
        SET approval_status = $1, approved_by = $2, approved_at = NOW(), updated_at = NOW()
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(status)
    .bind(user.id)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": format!("Blog post {}", payload.status),
        "data": post,
    })))
}

// POST /api/blog/:id/collaborators - Add collaborator (only author or TopLead)
pub async fn add_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let user_id: Uuid = serde_json::from_value(payload["user_id"].clone())
        .map_err(|_| ApiError::BadRequest("user_id is required".to_string()))?;
    
    let post: BlogPost = sqlx::query_as("SELECT * FROM blog_posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Check permissions: TopLead or author
    let is_author = post.author_id == Some(user.id);
    let can_add = user.has_role(&UserRole::TopLead) || is_author;
    
    if !can_add {
        return Err(ApiError::Forbidden("Only post author or TopLead can add collaborators".to_string()));
    }
    
    // Check if user exists
    let user_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    
    if !user_exists {
        return Err(ApiError::NotFound("User not found".to_string()));
    }
    
    // Add collaborator
    let collaborator: BlogCollaborator = sqlx::query_as(
        r#"
        INSERT INTO blog_collaborators (blog_post_id, user_id, added_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (blog_post_id, user_id) DO NOTHING
        RETURNING *
        "#
    )
    .bind(id)
    .bind(user_id)
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Collaborator added successfully",
            "data": collaborator,
        })),
    ))
}

// DELETE /api/blog/:id/collaborators/:user_id - Remove collaborator (only author or TopLead)
pub async fn remove_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path((id, collaborator_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let post: BlogPost = sqlx::query_as("SELECT * FROM blog_posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog post not found".to_string()))?;
    
    // Check permissions: TopLead or author
    let is_author = post.author_id == Some(user.id);
    let can_remove = user.has_role(&UserRole::TopLead) || is_author;
    
    if !can_remove {
        return Err(ApiError::Forbidden("Only post author or TopLead can remove collaborators".to_string()));
    }
    
    let deleted = sqlx::query(
        "DELETE FROM blog_collaborators WHERE blog_post_id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(collaborator_user_id)
    .execute(&state.db)
    .await?
    .rows_affected() > 0;
    
    if !deleted {
        return Err(ApiError::NotFound("Collaborator not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "Collaborator removed successfully",
    })))
}

// Helper function to check if user is a collaborator
async fn is_collaborator(db: &sqlx::PgPool, blog_post_id: Uuid, user_id: Uuid) -> Result<bool> {
    let is_collab: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blog_collaborators WHERE blog_post_id = $1 AND user_id = $2)"
    )
    .bind(blog_post_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    
    Ok(is_collab)
}
