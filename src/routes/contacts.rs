use axum::{extract::{Query, State}, http::StatusCode, Json};
use validator::Validate;
use crate::{
    dto::{contact_dto::CreateContactDto, response_dto::ApiResponse, pagination_dto::{PaginationParams, PaginationMeta}}, 
    error::Result, 
    repositories::contact_repository, 
    services::email_service::EmailService,
    AppState
};

pub async fn create_contact(
    State(state): State<AppState>,
    Json(payload): Json<CreateContactDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    
    let email = crate::utils::sanitize::normalize_email(&payload.email);
    let contact = contact_repository::create(
        &state.db, &payload.name, &email, &payload.subject, &payload.message,
    ).await?;
    
    // Send notification to admin
    if let Ok(email_service) = EmailService::new() {
        let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@socs.network".to_string());
        
        if let Err(e) = email_service.send_contact_form_notification(
            &admin_email,
            &payload.name,
            &payload.email,
            &payload.subject,
            &payload.message,
        ).await {
            tracing::error!("Failed to send contact form notification: {}", e);
        }
    } else {
        tracing::error!("Failed to initialize EmailService");
    }
    
    Ok((StatusCode::CREATED, ApiResponse::success_with_message("Contact submitted successfully", contact)))
}

// Admin only - list all contacts paginated
pub async fn list_contacts(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<impl axum::response::IntoResponse> {
    params.validate();
    
    let total = contact_repository::count_all(&state.db).await?;
    let contacts = contact_repository::find_all_paginated(&state.db, params.limit, params.offset()).await?;
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(ApiResponse::success_paginated(contacts, pagination))
}
