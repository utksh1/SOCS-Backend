import re

with open('src/routes/upload.rs', 'r') as f:
    content = f.read()

# Fix upload_image
new_upload = """pub async fn upload_image(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse> {
    let (data, content_type) = extract_validated_image(&mut multipart).await?;
    let r2_client = R2Client::new()?;
    
    let url = r2_client.upload_image(data.to_vec(), &content_type, "images")
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload image to R2: {}", e);
            ApiError::InternalServerError
        })?;
        
    sqlx::query!(
        "INSERT INTO uploaded_images (uploaded_by, url) VALUES ($1, $2)",
        user.id,
        url
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert into uploaded_images: {}", e);
        ApiError::InternalServerError
    })?;
    
    Ok(crate::dto::response_dto::ApiResponse::success(json!({"url": url})))
}"""

content = re.sub(r'pub async fn upload_image.*?Ok\(Json\(json!\(\{\n\s*"success": true,\n\s*"url": url\n\s*\}\)\)\)\n}', new_upload, content, flags=re.DOTALL)

# Fix upload_profile_picture
new_profile_upload = """pub async fn upload_profile_picture(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    mut multipart: Multipart,
) -> Result<impl axum::response::IntoResponse> {
    let (data, content_type) = extract_validated_image(&mut multipart).await?;
    let r2_client = R2Client::new()?;
    
    let url = r2_client.upload_image(data.to_vec(), &content_type, "profiles")
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload profile picture to R2: {}", e);
            ApiError::InternalServerError
        })?;
    
    let updated_user = user_repository::update_profile_picture(&state.db, user.id, &url).await?;
    
    Ok(crate::dto::response_dto::ApiResponse::success(json!({"url": url, "user": updated_user})))
}"""
content = re.sub(r'pub async fn upload_profile_picture.*?Ok\(Json\(json!\(\{\n\s*"success": true,\n\s*"message": "Profile picture uploaded successfully",\n\s*"url": url,\n\s*"data": updated_user\n\s*\}\)\)\)\n}', new_profile_upload, content, flags=re.DOTALL)

# Fix delete_image
new_delete = """pub async fn delete_image(
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
    
    user_repository::clear_profile_picture(&state.db, user.id).await?;
    
    let r2_client = R2Client::new()?;
    if let Err(e) = r2_client.delete_image(&payload.url).await {
        tracing::warn!("Failed to delete image from R2, but DB reference was cleared: {}", e);
    }
    
    Ok(crate::dto::response_dto::ApiResponse::success_with_message("Profile picture deleted successfully", json!({})))
}"""
content = re.sub(r'pub async fn delete_image.*?Ok\(Json\(json!\(\{\n\s*"success": true,\n\s*"message": "Image deleted successfully"\n\s*\}\)\)\)\n}', new_delete, content, flags=re.DOTALL)

with open('src/routes/upload.rs', 'w') as f:
    f.write(content)
