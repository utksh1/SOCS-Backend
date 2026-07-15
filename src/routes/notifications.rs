use axum::{extract::{Path, State}, Extension, Json};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::Result,
    models::user::SafeUser,
    repositories::{notification_repository, announcement_repository},
    AppState,
};

// Get user's notifications
pub async fn get_notifications(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    let notifications = notification_repository::find_by_user(&state.db, user.id, 50).await?;
    
    Ok(Json(json!({
        "success": true,
        "data": notifications
    })))
}

// Get unread count
pub async fn get_unread_count(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    let count = notification_repository::count_unread(&state.db, user.id).await?;
    
    Ok(Json(json!({
        "success": true,
        "count": count
    })))
}

// Mark notification as read
pub async fn mark_as_read(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    notification_repository::mark_as_read(&state.db, id, user.id).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Notification marked as read"
    })))
}

// Mark all as read
pub async fn mark_all_as_read(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    notification_repository::mark_all_as_read(&state.db, user.id).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "All notifications marked as read"
    })))
}

// Delete notification
pub async fn delete_notification(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    notification_repository::delete(&state.db, id, user.id).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Notification deleted"
    })))
}

// Get all announcements (public)
pub async fn get_announcements(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let announcements = announcement_repository::find_all(&state.db).await?;
    
    Ok(Json(json!({
        "success": true,
        "data": announcements
    })))
}

// Get single announcement (public)
pub async fn get_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let announcement = announcement_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Announcement not found".to_string()))?;
    
    Ok(Json(json!({
        "success": true,
        "data": announcement
    })))
}

// Create announcement (admin only)
pub async fn create_announcement(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let title = payload["title"].as_str().unwrap_or("");
    let content = payload["content"].as_str().unwrap_or("");
    let category = payload["category"].as_str().unwrap_or("general");
    let pinned = payload["pinned"].as_bool().unwrap_or(false);
    
    let announcement = announcement_repository::create(
        &state.db,
        title,
        content,
        category,
        pinned,
        user.id,
    ).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Announcement created",
        "data": announcement
    })))
}

// Update announcement (admin only)
pub async fn update_announcement(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let title = payload["title"].as_str().unwrap_or("");
    let content = payload["content"].as_str().unwrap_or("");
    let category = payload["category"].as_str().unwrap_or("general");
    let pinned = payload["pinned"].as_bool().unwrap_or(false);
    
    let announcement = announcement_repository::update(
        &state.db,
        id,
        title,
        content,
        category,
        pinned,
    ).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Announcement updated",
        "data": announcement
    })))
}

// Toggle pin (admin only)
pub async fn toggle_pin_announcement(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    announcement_repository::toggle_pin(&state.db, id).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Announcement pin toggled"
    })))
}

// Delete announcement (admin only)
pub async fn delete_announcement(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    announcement_repository::delete(&state.db, id).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Announcement deleted"
    })))
}
