use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        project_dto::{CreateProjectDto, UpdateProjectDto},
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result,
    models::user::SafeUser,
    repositories::project_repository,
    utils::slugify::slugify,
    AppState,
};

// GET /api/projects (with pagination)
pub async fn list_projects(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated projects
    let projects: Vec<crate::models::project::Project> = sqlx::query_as(
        r#"
        SELECT id, slug, title, description, tech_stack as tags, github_link,
               tags, featured, created_by, created_at, updated_at
        FROM projects
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
        "data": projects,
        "pagination": pagination
    })))
}

// GET /api/projects/:id
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Project not found".to_string()))?;
    
    Ok(Json(json!({
        "success": true,
        "data": project,
    })))
}

// POST /api/projects (protected - admin only)
pub async fn create_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateProjectDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.title));
    
    // Check if slug exists
    if project_repository::find_by_slug(&state.db, &slug).await?.is_some() {
        return Err(crate::error::ApiError::Conflict("Project slug already exists".to_string()));
    }
    
    let project = project_repository::create(
        &state.db,
        &slug,
        &payload.title,
        &payload.description,
        &payload.tech_stack,
        payload.github_link.as_deref(),
        &payload.tags,
        payload.featured.unwrap_or(false),
        Some(user.id),
    )
    .await?;
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Project created successfully",
            "data": project,
        })),
    ))
}

// PUT /api/projects/:id (protected - admin only)
pub async fn update_project(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProjectDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Check if project exists
    project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Project not found".to_string()))?;
    
    let project = project_repository::update(
        &state.db,
        id,
        payload.slug.as_deref(),
        payload.title.as_deref(),
        payload.description.as_deref(),
        payload.tech_stack.as_deref(),
        payload.github_link.as_ref().map(|s| Some(s.as_str())),
        payload.tags.as_deref(),
        payload.featured,
    )
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Project updated successfully",
        "data": project,
    })))
}

// DELETE /api/projects/:id (protected - admin only)
pub async fn delete_project(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let deleted = project_repository::delete(&state.db, id).await?;
    
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Project not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "Project deleted successfully",
    })))
}
