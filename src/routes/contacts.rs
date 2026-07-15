use axum::{extract::State, http::StatusCode, Json};
use serde_json::json;
use validator::Validate;
use crate::{
    dto::contact_dto::CreateContactDto, 
    error::Result, 
    repositories::contact_repository, 
    services::email_service::EmailService,
    AppState
};

pub async fn create_contact(
    State(state): State<AppState>,
    Json(payload): Json<CreateContactDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    let contact = contact_repository::create(
        &state.db, &payload.name, &payload.email, &payload.subject, &payload.message,
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
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Contact submitted successfully", "data": contact}))))
}

// Admin only - list all contacts
pub async fn list_contacts(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let contacts = contact_repository::find_all(&state.db).await?;
    Ok(Json(json!({"success": true, "data": contacts})))
}
