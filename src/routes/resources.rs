use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{
    dto::{
        resource_dto::CreateResourceDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::{ApiError, Result},
    models::{
        user::{SafeUser, UserRole},
        resource::{Resource, ContentStatus, ResourceCollaborator},
    },
    repositories::resource_repository,
    AppState
};

#[derive(Debug, Deserialize)]
pub struct ApprovalDto {
    pub status: String, // "approved" or "rejected"
}

// GET /api/resources - List approved resources (public, with pagination)
pub async fn list_resources(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count of approved resources
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources WHERE status = 'approved'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get paginated approved resources
    let resources: Vec<Resource> = sqlx::query_as(
        "SELECT * FROM resources WHERE status = 'approved' ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(params.limit)
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;
    
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(Json(json!({
        "success": true,
        "data": resources,
        "pagination": pagination
    })))
}

// GET /api/resources/all - List all resources including pending (TopLead only)
pub async fn list_all_resources(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    // Only TopLead can see all resources
    if !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden("Only TopLead can view all resources".to_string()));
    }
    
    params.validate();
    
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
        .fetch_one(&state.db)
        .await?;
    
    let resources: Vec<Resource> = sqlx::query_as(
        "SELECT * FROM resources ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(params.limit)
    .bind(params.offset())
    .fetch_all(&state.db)
    .await?;
    
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(Json(json!({
        "success": true,
        "data": resources,
        "pagination": pagination
    })))
}

// GET /api/resources/my - List user's own resources
pub async fn list_my_resources(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    let resources: Vec<Resource> = sqlx::query_as(
        "SELECT * FROM resources WHERE created_by = $1 ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": resources,
    })))
}

// GET /api/resources/:id - Get resource (public if approved, or if user is creator/collaborator)
pub async fn get_resource(
    State(state): State<AppState>,
    Extension(user): Extension<Option<SafeUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let resource: Resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;
    
    // Check if user can view this resource
    let can_view = match &user {
        Some(u) => {
            // TopLead can see everything
            u.has_role(&UserRole::TopLead) ||
            // Creator can see their own
            resource.created_by == Some(u.id) ||
            // Resource is approved (public)
            resource.status == ContentStatus::Approved ||
            // Check if user is a collaborator
            is_collaborator(&state.db, id, u.id).await?
        },
        None => resource.status == ContentStatus::Approved,
    };
    
    if !can_view {
        return Err(ApiError::NotFound("Resource not found".to_string()));
    }
    
    // Get collaborators
    let collaborators: Vec<ResourceCollaborator> = sqlx::query_as(
        "SELECT * FROM resource_collaborators WHERE resource_id = $1"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "resource": resource,
            "collaborators": collaborators,
        }
    })))
}

// POST /api/resources - Create resource (any authenticated user, starts as pending)
pub async fn create_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateResourceDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    // Create with pending status (TopLead can create approved directly)
    let status = if user.has_role(&UserRole::TopLead) {
        ContentStatus::Approved
    } else {
        ContentStatus::Pending
    };
    
    let resource: Resource = sqlx::query_as(
        r#"
        INSERT INTO resources (title, description, category, url, tags, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.category)
    .bind(&payload.url)
    .bind(&payload.tags)
    .bind(status)
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": if resource.status == ContentStatus::Approved {
                "Resource created and approved"
            } else {
                "Resource submitted for approval"
            },
            "data": resource
        }))
    ))
}

// PUT /api/resources/:id - Update resource (only creator, collaborators, or TopLead)
pub async fn update_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateResourceDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    let resource: Resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;
    
    // Check permissions: TopLead, creator, or collaborator
    let is_creator = resource.created_by == Some(user.id);
    let is_collab = is_collaborator(&state.db, id, user.id).await?;
    let can_edit = user.has_role(&UserRole::TopLead) || is_creator || is_collab;
    
    if !can_edit {
        return Err(ApiError::Forbidden("You don't have permission to edit this resource".to_string()));
    }
    
    // Update the resource
    let updated_resource: Resource = sqlx::query_as(
        r#"
        UPDATE resources
        SET title = $1, description = $2, category = $3, url = $4, tags = $5, updated_at = NOW()
        WHERE id = $6
        RETURNING *
        "#
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.category)
    .bind(&payload.url)
    .bind(&payload.tags)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Resource updated successfully",
        "data": updated_resource,
    })))
}

// DELETE /api/resources/:id - Delete resource (only creator or TopLead)
pub async fn delete_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let resource: Resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;
    
    // Check permissions: TopLead or creator
    let is_creator = resource.created_by == Some(user.id);
    let can_delete = user.has_role(&UserRole::TopLead) || is_creator;
    
    if !can_delete {
        return Err(ApiError::Forbidden("You don't have permission to delete this resource".to_string()));
    }
    
    let deleted = resource_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Resource not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "Resource deleted"
    })))
}

// POST /api/resources/:id/approve - Approve or reject resource (TopLead only)
pub async fn approve_or_reject_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ApprovalDto>,
) -> Result<Json<serde_json::Value>> {
    // Only TopLead can approve/reject
    if !user.has_role(&UserRole::TopLead) {
        return Err(ApiError::Forbidden("Only TopLead can approve or reject resources".to_string()));
    }
    
    let status = match payload.status.as_str() {
        "approved" => ContentStatus::Approved,
        "rejected" => ContentStatus::Rejected,
        _ => return Err(ApiError::BadRequest("Invalid status. Use 'approved' or 'rejected'".to_string())),
    };
    
    let resource: Resource = sqlx::query_as(
        r#"
        UPDATE resources
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
        "message": format!("Resource {}", payload.status),
        "data": resource,
    })))
}

// POST /api/resources/:id/collaborators - Add collaborator (only creator or TopLead)
pub async fn add_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let user_id: Uuid = serde_json::from_value(payload["user_id"].clone())
        .map_err(|_| ApiError::BadRequest("user_id is required".to_string()))?;
    
    let resource: Resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;
    
    // Check permissions: TopLead or creator
    let is_creator = resource.created_by == Some(user.id);
    let can_add = user.has_role(&UserRole::TopLead) || is_creator;
    
    if !can_add {
        return Err(ApiError::Forbidden("Only resource creator or TopLead can add collaborators".to_string()));
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
    let collaborator: ResourceCollaborator = sqlx::query_as(
        r#"
        INSERT INTO resource_collaborators (resource_id, user_id, added_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (resource_id, user_id) DO NOTHING
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

// DELETE /api/resources/:id/collaborators/:user_id - Remove collaborator (only creator or TopLead)
pub async fn remove_collaborator(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path((id, collaborator_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let resource: Resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;
    
    // Check permissions: TopLead or creator
    let is_creator = resource.created_by == Some(user.id);
    let can_remove = user.has_role(&UserRole::TopLead) || is_creator;
    
    if !can_remove {
        return Err(ApiError::Forbidden("Only resource creator or TopLead can remove collaborators".to_string()));
    }
    
    let deleted = sqlx::query(
        "DELETE FROM resource_collaborators WHERE resource_id = $1 AND user_id = $2"
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
async fn is_collaborator(db: &sqlx::PgPool, resource_id: Uuid, user_id: Uuid) -> Result<bool> {
    let is_collab: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM resource_collaborators WHERE resource_id = $1 AND user_id = $2)"
    )
    .bind(resource_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    
    Ok(is_collab)
}
