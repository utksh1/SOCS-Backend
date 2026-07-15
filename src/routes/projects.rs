use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        project_dto::{CreateProjectDto, UpdateProjectDto},
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::{ApiError, Result},
    models::{
        user::{SafeUser, UserRole},
        project::{Project, ContentStatus, ProjectCollaborator},
    },
    repositories::project_repository,
    utils::slugify::slugify,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ApprovalDto {
    pub status: String, // "approved" or "rejected"
}

// GET /api/projects (public - only shows approved projects)
pub async fn list_projects(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count of approved projects
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE status = 'approved'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get paginated approved projects
    let projects: Vec<Project> = sqlx::query_as(
        r#"
        SELECT * FROM projects
        WHERE status = 'approved'
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

// GET /api/projects/all (protected - TopLead only - shows all projects including pending)
pub async fn list_all_projects(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    // Only those who can approve content can see all projects
    crate::middleware::auth::can_approve_content(&user)?;
    
    params.validate();
    
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db)
        .await?;
    
    let projects: Vec<Project> = sqlx::query_as(
        "SELECT * FROM projects ORDER BY created_at DESC LIMIT $1 OFFSET $2"
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

// GET /api/projects/my (protected - shows user's own projects)
pub async fn list_my_projects(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    let projects: Vec<Project> = sqlx::query_as(
        "SELECT * FROM projects WHERE created_by = $1 ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": projects,
    })))
}

// GET /api/projects/:id (public - shows project if approved, or if user is owner/collaborator)
pub async fn get_project(
    State(state): State<AppState>,
    Extension(user): Extension<Option<SafeUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let project: Project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
    
    // Check if user can view this project
    let can_view = match &user {
        Some(u) => {
            // TopLead can see everything
            u.has_role(&UserRole::TopLead) ||
            // Creator can see their own
            project.created_by == Some(u.id) ||
            // Project is approved (public)
            project.status == ContentStatus::Approved ||
            // Check if user is a collaborator
            is_collaborator(&state.db, id, u.id).await?
        },
        None => project.status == ContentStatus::Approved,
    };
    
    if !can_view {
        return Err(ApiError::NotFound("Project not found".to_string()));
    }
    
    // Get collaborators
    let collaborators: Vec<ProjectCollaborator> = sqlx::query_as(
        "SELECT * FROM project_collaborators WHERE project_id = $1"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "project": project,
            "collaborators": collaborators,
        }
    })))
}

// POST /api/projects (protected - any authenticated user can create, starts as pending)
pub async fn create_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateProjectDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.title));
    
    // Check if slug exists
    if project_repository::find_by_slug(&state.db, &slug).await?.is_some() {
        return Err(ApiError::Conflict("Project slug already exists".to_string()));
    }
    
    // Create project with pending status (TopLead can create approved directly)
    let status = if user.has_role(&UserRole::TopLead) {
        ContentStatus::Approved
    } else {
        ContentStatus::Pending
    };
    
    let project: Project = sqlx::query_as(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, github_link, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(&slug)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.tech_stack)
    .bind(&payload.github_link)
    .bind(&payload.tags)
    .bind(payload.featured.unwrap_or(false))
    .bind(status)
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": if project.status == ContentStatus::Approved {
                "Project created and approved"
            } else {
                "Project submitted for approval"
            },
            "data": project,
        })),
    ))
}

// PUT /api/projects/:id (protected - only owner, collaborators, or TopLead can update)
pub async fn update_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProjectDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    let project: Project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
    
    // Check permissions: TopLead, owner, or collaborator
    let is_owner = project.created_by == Some(user.id);
    let is_collab = is_collaborator(&state.db, id, user.id).await?;
    let can_edit = user.has_role(&UserRole::TopLead) || is_owner || is_collab;
    
    if !can_edit {
        return Err(ApiError::Forbidden("You don't have permission to edit this project".to_string()));
    }
    
    let updated_project = project_repository::update(
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
        "data": updated_project,
    })))
}

// DELETE /api/projects/:id (protected - only owner or TopLead can delete)
pub async fn delete_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let project: Project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
    
    // Check permissions: TopLead or owner
    let is_owner = project.created_by == Some(user.id);
    let can_delete = user.has_role(&UserRole::TopLead) || is_owner;
    
    if !can_delete {
        return Err(ApiError::Forbidden("You don't have permission to delete this project".to_string()));
    }
    
    let deleted = project_repository::delete(&state.db, id).await?;
    
    if !deleted {
        return Err(ApiError::NotFound("Project not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "Project deleted successfully",
    })))
}

// POST /api/projects/:id/approve (protected - TopLead only)
pub async fn approve_or_reject_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ApprovalDto>,
) -> Result<Json<serde_json::Value>> {
    // Only TopLead can approve/reject
    crate::middleware::auth::can_approve_content(&user)?;
    
    let status = match payload.status.as_str() {
        "approved" => ContentStatus::Approved,
        "rejected" => ContentStatus::Rejected,
        _ => return Err(ApiError::BadRequest("Invalid status. Use 'approved' or 'rejected'".to_string())),
    };
    
    let project: Project = sqlx::query_as(
        r#"
        UPDATE projects
        SET status = $1, approved_by = $2, approved_at = NOW(), updated_at = NOW()
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
        "message": format!("Project {}", payload.status),
        "data": project,
    })))
}

// POST /api/projects/:id/collaborators (protected - only owner or TopLead)
pub async fn add_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let user_id: Uuid = serde_json::from_value(payload["user_id"].clone())
        .map_err(|_| ApiError::BadRequest("user_id is required".to_string()))?;
    
    let project: Project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
    
    // Check permissions: TopLead or owner
    let is_owner = project.created_by == Some(user.id);
    let can_add = user.has_role(&UserRole::TopLead) || is_owner;
    
    if !can_add {
        return Err(ApiError::Forbidden("Only project owner or TopLead can add collaborators".to_string()));
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
    let collaborator: ProjectCollaborator = sqlx::query_as(
        r#"
        INSERT INTO project_collaborators (project_id, user_id, added_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, user_id) DO NOTHING
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

// DELETE /api/projects/:id/collaborators/:user_id (protected - only owner or TopLead)
pub async fn remove_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path((id, collaborator_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let project: Project = project_repository::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
    
    // Check permissions: TopLead or owner
    let is_owner = project.created_by == Some(user.id);
    let can_remove = user.has_role(&UserRole::TopLead) || is_owner;
    
    if !can_remove {
        return Err(ApiError::Forbidden("Only project owner or TopLead can remove collaborators".to_string()));
    }
    
    let deleted = sqlx::query(
        "DELETE FROM project_collaborators WHERE project_id = $1 AND user_id = $2"
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
async fn is_collaborator(db: &sqlx::PgPool, project_id: Uuid, user_id: Uuid) -> Result<bool> {
    let is_collab: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM project_collaborators WHERE project_id = $1 AND user_id = $2)"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    
    Ok(is_collab)
}
