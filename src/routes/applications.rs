use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{
    dto::{
        application_dto::{CreateApplicationDto, ReviewApplicationDto},
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result, 
    models::user::SafeUser, 
    repositories::application_repository, 
    services::email_service::EmailService,
    AppState
};

// Public - submit application from /join page
pub async fn create_application(
    State(state): State<AppState>,
    Json(payload): Json<CreateApplicationDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    let application = application_repository::create(
        &state.db, &payload.name, &payload.email, &payload.experience_level, &payload.skills, payload.message.as_deref(),
    ).await?;
    
    // Send confirmation email (non-blocking, log errors but don't fail request)
    if let Ok(email_service) = EmailService::new() {
        if let Err(e) = email_service.send_application_received(
            &application.email,
            &application.name,
            &application.experience_level,
            &application.skills,
        ).await {
            tracing::error!("Failed to send application confirmation email: {}", e);
        }
    } else {
        tracing::error!("Failed to initialize EmailService");
    }
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Application submitted successfully", "data": application}))))
}

// Admin only - list all applications (with pagination)
pub async fn list_applications(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM applications")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated applications
    let applications: Vec<crate::models::application::Application> = sqlx::query_as(
        r#"
        SELECT id, name, email, experience_level, skills, message, status,
               created_at, reviewed_by, reviewed_at, rejection_reason
        FROM applications
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
        "data": applications,
        "pagination": pagination
    })))
}

// Admin only - get single application
pub async fn get_application(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    let application = application_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Application not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": application})))
}

// Admin only - review application (approve/reject)
pub async fn review_application(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReviewApplicationDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    let application = application_repository::review(
        &state.db, id, payload.status.clone(), user.id, payload.rejection_reason.as_deref(),
    ).await?;
    
    // Send approval/rejection email based on status
    if let Ok(email_service) = EmailService::new() {
        match &payload.status {
            crate::models::application::ApplicationStatus::Approved => {
                if let Err(e) = email_service.send_application_approved(
                    &application.email,
                    &application.name,
                ).await {
                    tracing::error!("Failed to send approval email: {}", e);
                }
            }
            crate::models::application::ApplicationStatus::Rejected => {
                if let Err(e) = email_service.send_application_rejected(
                    &application.email,
                    &application.name,
                ).await {
                    tracing::error!("Failed to send rejection email: {}", e);
                }
            }
            _ => {}
        }
    } else {
        tracing::error!("Failed to initialize EmailService");
    }
    
    Ok(Json(json!({"success": true, "message": "Application reviewed successfully", "data": application})))
}
