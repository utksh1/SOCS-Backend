use std::env;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Extension,
    Json,
};
use axum::body::Bytes;
use serde_json::json;
use validator::Validate;
use std::io::Cursor;
use image::ImageFormat;

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
            let data = field.bytes().await
                .map_err(|_| ApiError::BadRequest("Failed to read file data".to_string()))?;
            
            if data.len() > MAX_FILE_SIZE {
                return Err(ApiError::BadRequest("File size exceeds 5MB limit".to_string()));
            }
            
            // Magic byte detection
            let detected_type = infer::get(&data)
                .ok_or_else(|| ApiError::BadRequest("Could not determine file type".to_string()))?;
                
            let content_type = detected_type.mime_type();
            
            if !ALLOWED_TYPES.contains(&content_type) {
                return Err(ApiError::BadRequest(
                    "Only image files (JPEG, PNG, GIF, WebP) are allowed".to_string()
                ));
            }
            
            // Decode and re-encode image to strip EXIF data and neutralize polyglots
            let img = tokio::task::spawn_blocking(move || {
                image::load_from_memory(&data)
            })
            .await
            .map_err(|_| ApiError::InternalServerError)?
            .map_err(|e| {
                tracing::warn!("Failed to decode image: {:?}", e);
                ApiError::BadRequest("Invalid or corrupted image data".to_string())
            })?;
            
            let format = match content_type {
                "image/jpeg" => image::ImageFormat::Jpeg,
                "image/png" => image::ImageFormat::Png,
                "image/gif" => image::ImageFormat::Gif,
                "image/webp" => image::ImageFormat::WebP,
                _ => image::ImageFormat::Jpeg,
            };
            
            let safe_data = tokio::task::spawn_blocking(move || {
                let mut buffer = std::io::Cursor::new(Vec::new());
                img.write_to(&mut buffer, format).map(|_| buffer.into_inner())
            })
            .await
            .map_err(|_| ApiError::InternalServerError)?
            .map_err(|e| {
                tracing::error!("Failed to re-encode image: {:?}", e);
                ApiError::InternalServerError
            })?;
            
            return Ok((axum::body::Bytes::from(safe_data), content_type.to_string()));
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
    
    let r2_client = R2Client::new()?;
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
    
    let r2_client = R2Client::new()?;
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
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<DeleteImageDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    
    if !payload.url.contains(&env::var("R2_PUBLIC_URL").unwrap_or_default()) {
        return Err(ApiError::BadRequest("Invalid image URL".to_string()));
    }
    
    let current_user = user_repository::find_by_id(&state.db, user.id).await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
        
    if current_user.profile_picture.as_deref() != Some(&payload.url) && current_user.avatar_url.as_deref() != Some(&payload.url) {
        return Err(ApiError::Forbidden("You can only delete your own profile picture".to_string()));
    }
    
    user_repository::clear_profile_picture(&state.db, user.id, &payload.url).await?;
    
    let r2_client = R2Client::new()?;
    if let Err(e) = r2_client.delete_image(&payload.url).await {
        tracing::warn!("Failed to delete image from R2, but DB reference was cleared: {}", e);
    }
    
    Ok(crate::dto::response_dto::ApiResponse::success_with_message("Profile picture deleted successfully", json!({})))
}

pub fn upload_body_limit() -> axum::extract::DefaultBodyLimit {
    axum::extract::DefaultBodyLimit::max(5 * 1024 * 1024)
}
