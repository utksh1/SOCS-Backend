import os

with open('src/routes/upload.rs', 'r') as f:
    content = f.read()

# Add magic byte detection and EXIF stripping to extract_validated_image
new_extract = """async fn extract_validated_image(multipart: &mut Multipart) -> Result<(Bytes, String)> {
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
}"""
import re
content = re.sub(r'async fn extract_validated_image.*?Err\(ApiError::BadRequest\("No image file provided"\.to_string\(\)\)\)\n}', new_extract, content, flags=re.DOTALL)

with open('src/routes/upload.rs', 'w') as f:
    f.write(content)
