use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    models::user::{SafeUser, UserRole},
    services::{cleanup_service, event_service, project_service, user_service},
    AppState,
};

/// Helper function to check if user has admin permissions
fn require_admin(user: &SafeUser) -> Result<()> {
    // TopLead and Mentor have admin capabilities
    if user.has_role(&UserRole::TopLead) || user.has_role(&UserRole::Mentor) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Admin or management role required".to_string(),
        ))
    }
}

// ============================================================================
// Project Admin Handlers
// ============================================================================

pub async fn restore_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    project_service::restore_project(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Project restored successfully",
        "id": id
    })))
}

pub async fn permanent_delete_project(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    project_service::permanent_delete_project(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Project permanently deleted",
        "id": id
    })))
}

// ============================================================================
// Event Admin Handlers
// ============================================================================

pub async fn restore_event(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    event_service::restore_event(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Event restored successfully",
        "id": id
    })))
}

pub async fn permanent_delete_event(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    event_service::permanent_delete_event(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Event permanently deleted",
        "id": id
    })))
}

// ============================================================================
// User Admin Handlers
// ============================================================================

pub async fn restore_user(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    user_service::restore_user(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "User restored successfully",
        "id": id
    })))
}

pub async fn permanent_delete_user(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&user)?;

    user_service::permanent_delete_user(&state.db, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "User permanently deleted",
        "id": id
    })))
}

// ============================================================================
// Cleanup Handler
// ============================================================================

pub async fn run_manual_cleanup(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    require_admin(&user)?;

    let stats = cleanup_service::cleanup_expired_soft_deletes(&state.db).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Cleanup completed successfully",
            "stats": stats
        })),
    ))
}

// ============================================================================
// Route Configuration
// ============================================================================

use axum::Router;

pub fn configure() -> Router<AppState> {
    Router::new()
        // Project admin routes
        .route(
            "/projects/:id/restore",
            axum::routing::post(restore_project),
        )
        .route(
            "/projects/:id/permanent",
            axum::routing::delete(permanent_delete_project),
        )
        // Event admin routes
        .route("/events/:id/restore", axum::routing::post(restore_event))
        .route(
            "/events/:id/permanent",
            axum::routing::delete(permanent_delete_event),
        )
        // User admin routes
        .route("/users/:id/restore", axum::routing::post(restore_user))
        .route(
            "/users/:id/permanent",
            axum::routing::delete(permanent_delete_user),
        )
        // Cleanup route
        .route("/cleanup/run", axum::routing::post(run_manual_cleanup))
}
