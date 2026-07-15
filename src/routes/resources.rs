use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{
    dto::{
        resource_dto::CreateResourceDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result,
    models::user::SafeUser,
    repositories::resource_repository,
    AppState
};

pub async fn list_resources(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated resources
    let resources: Vec<crate::models::resource::Resource> = sqlx::query_as(
        r#"
        SELECT id, title, description, category, url, tags, created_by, created_at, updated_at
        FROM resources
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
        "data": resources,
        "pagination": pagination
    })))
}

pub async fn get_resource(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    let resource = resource_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Resource not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": resource})))
}

pub async fn create_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateResourceDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // Check permission
    
    payload.validate()?;
    
    let resource = resource_repository::create(
        &state.db, &payload.title, &payload.description, payload.category, &payload.url, &payload.tags, Some(user.id),
    ).await?;
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Resource created", "data": resource}))))
}

pub async fn delete_resource(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check permission
    
    let deleted = resource_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Resource not found".to_string()));
    }
    Ok(Json(json!({"success": true, "message": "Resource deleted"})))
}
