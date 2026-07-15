use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Extension,
    Json,
};
use axum::body::Bytes;
use serde_json::json;
use validator::Validate;

use crate::{
    dto::upload_dto::DeleteImageDto,
    error::{ApiError, Result},
    models::user::SafeUser,
    services::r2_service::R2Client,
    repositories::user_repository,
    AppState,
};

const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

// Extract and validate image from multipart data
async fn extract_validated_image(multipart: &mut Multipart) -> Result<(Bytes, String)> {
    while let Some(field) = multipart.next_field().await
        .map_err(|_| ApiError::BadRequest("Invalid multipart data".to_string()))? {
        
        if field.name().unwrap_or("") == "image" {
            let content_type = field.content_type()
                .ok_or_else(|| ApiError::BadRequest("Missing content type".to_string()))?
                .to_string();
            
            if !ALLOWED_TYPES.contains(&content_type.as_str()) {
                return Err(ApiError::BadRequest(
                    "Only image files (JPEG, PNG, GIF, WebP) are allowed".to_string()
                ));
            }
            
            let data = field.bytes().await
                .map_err(|_| ApiError::BadRequest("Failed to read file data".to_string()))?;
            
            if data.len() > MAX_FILE_SIZE {
                return Err(ApiError::BadRequest("File size exceeds 5MB limit".to_string()));
            }
            
            return Ok((data, content_type));
        }
    }
    
    Err(ApiError::BadRequest("No image file provided".to_string()))
}

pub async fn upload_image(
    State(_state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let (data, content_type) = extract_validated_image(&mut multipart).await?;
    
    let r2_client = R2Client::new();
    let url = r2_client.upload_image(data.to_vec(), &content_type, "images")
        .await
        .map_err(|_| ApiError::InternalServerError)?;
    
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Image uploaded successfully",
            "data": { "url": url }
        }))
    ))
}

pub async fn upload_profile_picture(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let (data, content_type) = extract_validated_image(&mut multipart).await?;
    
    let r2_client = R2Client::new();
    let url = r2_client.upload_image(data.to_vec(), &content_type, "profiles")
        .await
        .map_err(|_| ApiError::InternalServerError)?;
    
    user_repository::update_profile_picture(&state.db, user.id, &url).await?;
    
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Profile picture updated",
            "data": { "url": url }
        }))
    ))
}

pub async fn delete_image(
    State(_state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Json(payload): Json<DeleteImageDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;
    
    let r2_client = R2Client::new();
    r2_client.delete_image(&payload.url)
        .await
        .map_err(|_| ApiError::InternalServerError)?;
    
    Ok(Json(json!({
        "success": true,
        "message": "Image deleted successfully"
    })))
}
