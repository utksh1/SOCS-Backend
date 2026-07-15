import re

with open('src/routes/upload.rs', 'r') as f:
    content = f.read()

content = content.replace(
"""return Ok((data, detected_type.to_string()));""",
"""let detected_type_str = detected_type.to_string();
            
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
            
            let format = match detected_type_str.as_str() {
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
            
            return Ok((axum::body::Bytes::from(safe_data), detected_type_str));""")

with open('src/routes/upload.rs', 'w') as f:
    f.write(content)
