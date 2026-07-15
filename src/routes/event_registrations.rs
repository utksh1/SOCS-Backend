use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{
    dto::event_registration_dto::RegisterForEventDto, 
    error::Result, 
    models::user::SafeUser, 
    repositories::{event_registration_repository, event_repository},
    services::email_service::EmailService,
    AppState
};

// POST /api/events/:id/register - Register for event (public)
pub async fn register_for_event(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<RegisterForEventDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    // Check if event exists and get event details
    let event = event_repository::find_by_id(&state.db, event_id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Event not found".to_string()))?;
    
    let registration = event_registration_repository::register(
        &state.db,
        event_id,
        &payload.name,
        &payload.email,
        None,
    ).await.map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            crate::error::ApiError::Conflict("You are already registered for this event".to_string())
        } else {
            crate::error::ApiError::InternalServerError
        }
    })?;
    
    // Send confirmation email
    if let Ok(email_service) = EmailService::new() {
        let event_date = event.date.format("%B %d, %Y at %I:%M %p").to_string();
        let event_location = event.location.as_deref().unwrap_or("TBA");
        
        if let Err(e) = email_service.send_event_registration_confirmation(
            &payload.email,
            &payload.name,
            &event.title,
            &event_date,
            event_location,
        ).await {
            tracing::error!("Failed to send event registration confirmation: {}", e);
        }
    } else {
        tracing::error!("Failed to initialize EmailService");
    }
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Registered successfully", "data": registration}))))
}

// GET /api/events/:id/registrations - List registrations (admin)
pub async fn list_registrations(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let registrations = event_registration_repository::find_by_event(&state.db, event_id).await?;
    let count = event_registration_repository::count_by_event(&state.db, event_id).await?;
    
    Ok(Json(json!({"success": true, "data": registrations, "count": count})))
}
