use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{dto::visual_dto::CreateVisualDto, error::Result, models::user::SafeUser, repositories::visual_repository, AppState};

pub async fn list_visuals(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let visuals = visual_repository::find_all(&state.db).await?;
    Ok(Json(json!({"success": true, "data": visuals})))
}

pub async fn get_visual(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    let visual = visual_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Visual not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": visual})))
}

pub async fn create_visual(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateVisualDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    let visual = visual_repository::create(
        &state.db, &payload.title, payload.category, &payload.src, payload.alt_text.as_deref(), Some(user.id),
    ).await?;
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Visual created", "data": visual}))))
}

pub async fn delete_visual(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let deleted = visual_repository::soft_delete(&state.db, id).await?;
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Visual not found".to_string()));
    }
    Ok(Json(json!({"success": true, "message": "Visual deleted"})))
}
