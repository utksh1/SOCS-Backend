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
        event_dto::CreateEventDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result,
    models::user::SafeUser,
    repositories::event_repository,
    utils::slugify::slugify,
    AppState,
};

pub async fn list_events(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated events
    let events: Vec<crate::models::event::Event> = sqlx::query_as(
        r#"
        SELECT id, slug, title, description, date, type as event_type, status,
               location, created_by as organizer_id, created_at, updated_at
        FROM events
        ORDER BY date DESC
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
        "data": events,
        "pagination": pagination
    })))
}

pub async fn get_event(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    let event = event_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Event not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": event})))
}

pub async fn create_event(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateEventDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // Check permission
    
    payload.validate()?;
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.title));
    
    let event = event_repository::create(
        &state.db, &slug, &payload.title, &payload.description, payload.date,
        payload.event_type, payload.status.unwrap_or(crate::models::event::EventStatus::Upcoming),
        payload.location.as_deref(), Some(user.id),
    ).await?;
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Event created", "data": event}))))
}

pub async fn delete_event(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check permission
    
    let deleted = event_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Event not found".to_string()));
    }
    Ok(Json(json!({"success": true, "message": "Event deleted"})))
}
