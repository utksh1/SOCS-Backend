# Secure File Upload System Design

**Date:** 2026-07-15  
**Status:** Approved  
**Author:** SOCS Backend Team

## Overview

This document specifies the security enhancements for the SOCS backend file upload system. The current implementation accepts image uploads via multipart forms and stores them in Cloudflare R2, but relies solely on client-provided content-type headers for validation. This design adds defense-in-depth through magic number validation, EXIF stripping, and rate limiting.

## Problem Statement

The current upload system has three security gaps:

1. **Content-Type Spoofing:** Attackers can upload malicious files disguised as images by manipulating HTTP headers
2. **Privacy Leaks:** Uploaded images may contain EXIF metadata (GPS coordinates, camera info, timestamps)
3. **Abuse Potential:** No rate limiting allows users to spam uploads

## Goals

1. Validate actual file content using magic numbers (file signatures)
2. Strip EXIF and other metadata from uploaded images through reprocessing
3. Implement per-user rate limiting (10 uploads per hour)
4. Maintain existing functionality and API contracts
5. Keep upload latency reasonable (target: <2s for 5MB image)

## Non-Goals

- Storage quotas per user (may add later)
- Virus scanning (out of scope)
- Support for non-image file types
- CDN integration or image optimization

## Architecture

### High-Level Design

We implement a service-oriented architecture with three main components:

1. **ImageProcessingService** - Validates file types and sanitizes images
2. **RateLimitMiddleware** - Enforces upload rate limits per authenticated user
3. **Enhanced Upload Routes** - Simplified handlers that delegate to services

### Data Flow

```
Client Request 
  ↓
Rate Limit Middleware (check user hasn't exceeded 10 uploads/hour)
  ↓
Auth Middleware (existing - verify JWT token)
  ↓
Upload Handler
  ↓
Extract Multipart Data (raw bytes + claimed content-type)
  ↓
Validate File Size (existing 5MB limit)
  ↓
ImageProcessingService.process_upload()
  ├─ Detect actual file type via magic numbers (infer crate)
  ├─ Verify detected type matches claimed type
  ├─ Decode image to validate format (image crate)
  └─ Re-encode to strip EXIF and sanitize
  ↓
Upload sanitized bytes to Cloudflare R2
  ↓
Return public URL to client
```

### Security Layers

1. **Rate Limiting:** Prevents abuse (10 uploads/hour per user)
2. **Authentication:** Existing JWT middleware ensures only authenticated users upload
3. **Content-Type Validation:** Existing header check + new magic number validation
4. **Magic Number Validation:** Verify first bytes match image signatures (prevents spoofing)
5. **Image Reprocessing:** Decode and re-encode strips EXIF and validates format integrity
6. **Size Limits:** Existing 5MB maximum file size

## Component Specifications

### 1. ImageProcessingService

**Location:** `src/services/image_processing_service.rs`

**Purpose:** Encapsulates all image validation, sanitization, and processing logic.

**Dependencies:**
- `infer` crate (v0.15+) - Magic number detection
- `image` crate (v0.25+) - Image decoding/encoding with format support

**Public API:**

```rust
pub struct ImageProcessingService;

impl ImageProcessingService {
    /// Validates file content using magic numbers
    /// Returns detected MIME type or error if not a supported image
    pub fn validate_file_type(data: &[u8]) -> Result<String, ApiError>;
    
    /// Processes image: validates, decodes, strips EXIF, re-encodes
    /// Returns (sanitized_bytes, confirmed_mime_type)
    pub fn process_upload(
        raw_data: Vec<u8>, 
        claimed_type: &str
    ) -> Result<(Vec<u8>, String), ApiError>;
}
```

**Implementation Details:**

**validate_file_type():**
1. Use `infer::get(data)` to detect file type from magic numbers
2. Check if detected type is in allowed list: `image/jpeg`, `image/png`, `image/gif`, `image/webp`
3. Return detected MIME type or error

**process_upload():**
1. Call `validate_file_type()` to get actual file type
2. Verify detected type matches claimed content-type (case-insensitive comparison)
3. Use `image::load_from_memory()` to decode the image:
   - This validates the file is a well-formed image
   - Automatically handles format detection
   - Returns `DynamicImage` in memory
4. Re-encode based on detected format:
   - JPEG: Use `JpegEncoder` with quality 95
   - PNG: Use `PngEncoder`
   - GIF: Use `GifEncoder`
   - WebP: Use `WebPEncoder`
5. Write encoded bytes to `Vec<u8>` buffer
6. Return sanitized bytes + confirmed MIME type

**EXIF Stripping:** The `image` crate's decode → re-encode process automatically strips EXIF metadata, ICC profiles, and other ancillary chunks. This is the standard approach for image sanitization in Rust.

**Error Handling:**
- `ApiError::BadRequest` for invalid/unsupported file types
- `ApiError::BadRequest` for type mismatch (claimed vs detected)
- `ApiError::BadRequest` for corrupt images that fail to decode
- `ApiError::InternalServerError` for encoding failures

### 2. RateLimitMiddleware

**Location:** `src/middleware/rate_limit.rs`

**Purpose:** Prevents upload abuse by limiting uploads per user per time window.

**Configuration:**
- Limit: 10 uploads per user
- Window: 1 hour (3600 seconds)
- Tracking: In-memory HashMap (sufficient for single-instance deployment)

**State Structure:**

```rust
#[derive(Clone)]
pub struct RateLimitState {
    // Map: user_id -> (upload_count, window_start_time)
    uploads: Arc<RwLock<HashMap<Uuid, (u32, DateTime<Utc>)>>>,
}

impl RateLimitState {
    pub fn new() -> Self;
    
    /// Check if user can upload, increment counter if allowed
    /// Returns Ok(()) if allowed, Err if rate limit exceeded
    pub async fn check_and_increment(
        &self, 
        user_id: Uuid
    ) -> Result<(), ApiError>;
}
```

**Middleware Function:**

```rust
pub async fn rate_limit_uploads(
    Extension(rate_limit_state): Extension<Arc<RateLimitState>>,
    Extension(user): Extension<SafeUser>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError>;
```

**Implementation Logic:**

1. Extract user ID from `Extension<SafeUser>` (injected by auth middleware)
2. Acquire read lock on uploads HashMap
3. Check if user has entry:
   - **No entry:** Create new entry with count=1, current time
   - **Has entry:**
     - If window expired (>1 hour ago): Reset to count=1, current time
     - If within window and count < 10: Increment count
     - If within window and count >= 10: Return 429 error
4. Release lock and call `next.run(request).await` if allowed

**429 Response Format:**

```json
{
  "success": false,
  "message": "Rate limit exceeded. Maximum 10 uploads per hour.",
  "retry_after": 3456
}
```

Where `retry_after` is seconds until the window resets.

**Cleanup Strategy:** Simple time-based expiration on check. When acquiring write lock, entries with window_start > 1 hour ago are reset. No background cleanup task needed for this scale.

**Production Considerations:** For multi-instance deployments, replace in-memory HashMap with Redis-backed rate limiting. The interface remains the same.

### 3. Enhanced Upload Routes

**Location:** `src/routes/upload.rs`

**Changes:**

**New Helper Function:**

```rust
/// Extracts raw file data from multipart without validation
async fn extract_multipart_file(
    multipart: &mut Multipart
) -> Result<(Vec<u8>, String), ApiError>;
```

This replaces `extract_validated_image()` with simpler extraction logic. Validation moves to `ImageProcessingService`.

**Updated upload_image():**

```rust
pub async fn upload_image(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // 1. Extract raw data and claimed content-type
    let (raw_data, claimed_type) = extract_multipart_file(&mut multipart).await?;
    
    // 2. Validate size (existing 5MB check)
    if raw_data.len() > MAX_FILE_SIZE {
        return Err(ApiError::BadRequest("File exceeds 5MB limit".into()));
    }
    
    // 3. Process: validate + sanitize + strip EXIF
    let (sanitized_data, confirmed_type) = 
        ImageProcessingService::process_upload(raw_data, &claimed_type)?;
    
    // 4. Upload to R2
    let r2_client = R2Client::new();
    let url = r2_client.upload_image(
        sanitized_data, 
        &confirmed_type, 
        "images"
    ).await
        .map_err(|_| ApiError::InternalServerError)?;
    
    // 5. Return URL
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Image uploaded successfully",
            "data": { "url": url }
        }))
    ))
}
```

**Updated upload_profile_picture():**

Same structure as `upload_image()`, but:
- Uploads to "profiles" folder instead of "images"
- Calls `user_repository::update_profile_picture()` to update user record

**No changes to delete_image()** - deletion doesn't need processing.

### 4. Route Configuration

**Location:** `src/routes/mod.rs`

**Changes:**

Add rate limit state to AppState:

```rust
pub struct AppState {
    pub db: PgPool,
    pub rate_limit_state: Arc<RateLimitState>,
}
```

Configure upload routes with middleware:

```rust
let rate_limit_state = Arc::new(RateLimitState::new());

let app_state = AppState {
    db: pool.clone(),
    rate_limit_state: rate_limit_state.clone(),
};

// Upload routes with rate limiting
let upload_routes = Router::new()
    .route("/upload/image", post(upload::upload_image))
    .route("/upload/profile-picture", post(upload::upload_profile_picture))
    .route("/upload/delete", delete(upload::delete_image))
    .layer(Extension(rate_limit_state))
    .layer(middleware::from_fn_with_state(
        app_state.clone(),
        rate_limit::rate_limit_uploads
    ))
    .layer(middleware::from_fn_with_state(
        app_state.clone(),
        auth::auth_middleware
    ));
```

**Middleware Execution Order (outer to inner):**
1. Rate limit check
2. Authentication check  
3. Upload handler

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# Image processing
infer = "0.15"
image = { version = "0.25", features = ["jpeg", "png", "gif", "webp"] }
```

The `image` crate features enable format-specific encoders/decoders. Only include formats we support.

## Error Handling

### New Error Scenarios

1. **Magic Number Mismatch:**
   - Status: 400 Bad Request
   - Message: "File type mismatch. Claimed: {claimed}, Detected: {detected}"

2. **Unsupported File Type:**
   - Status: 400 Bad Request
   - Message: "Unsupported file type. Only JPEG, PNG, GIF, and WebP images are allowed."

3. **Corrupt Image:**
   - Status: 400 Bad Request
   - Message: "Failed to decode image. File may be corrupt."

4. **Rate Limit Exceeded:**
   - Status: 429 Too Many Requests
   - Message: "Rate limit exceeded. Maximum 10 uploads per hour."
   - Header: `Retry-After: {seconds}`

### Existing Errors (Maintained)

- Invalid multipart data (400)
- Missing content type (400)
- File too large (400)
- No image file provided (400)
- R2 upload failure (500)

## Testing Strategy

### Unit Tests

1. **ImageProcessingService:**
   - Test magic number detection for each supported format
   - Test rejection of non-image files (PDF, executable, text)
   - Test type mismatch detection (JPEG with PNG header)
   - Test EXIF stripping (verify re-encoded images lack metadata)
   - Test corrupt image handling

2. **RateLimitMiddleware:**
   - Test counter increment within window
   - Test window expiration and reset
   - Test 429 response when limit exceeded
   - Test concurrent requests from same user

### Integration Tests

1. **Upload Flow:**
   - Upload valid images (JPEG, PNG, GIF, WebP)
   - Verify returned URLs are accessible
   - Verify uploaded images lack EXIF data
   - Attempt spoofed file upload (should reject)
   - Test rate limiting (11th upload should fail)

2. **Error Scenarios:**
   - Upload non-image file
   - Upload corrupted image
   - Upload image with wrong content-type header
   - Exceed file size limit

### Manual Testing

1. Use ExifTool to verify uploaded images have no EXIF data
2. Test with images containing GPS coordinates, camera info
3. Verify performance with 5MB images (should complete <2s)

## Performance Considerations

### Processing Overhead

- **Magic number check:** ~1-5ms (reads first 512 bytes)
- **Image decode:** ~100-500ms for 5MB JPEG
- **Image re-encode:** ~200-800ms for 5MB JPEG
- **Total added latency:** ~300-1300ms

This is acceptable for the 5MB maximum size. Most uploads will be <1MB and complete faster.

### Memory Usage

- Peak memory: ~3x file size (raw bytes + decoded image + encoded output)
- Max: ~15MB for 5MB upload (within reasonable limits for Rust)

### Rate Limit Storage

- Per-user overhead: 32 bytes (Uuid + u32 + DateTime)
- 1000 active users: ~32KB
- HashMap lookup: O(1) average case

## Security Analysis

### Threat Model

**Threats Addressed:**

1. ✅ **Malicious File Upload:** Magic numbers prevent disguised executables
2. ✅ **Privacy Leak:** EXIF stripping removes GPS and camera metadata
3. ✅ **DoS via Upload Spam:** Rate limiting prevents abuse
4. ✅ **File Type Confusion:** Validation ensures files match claimed type

**Threats Not Addressed (Acknowledged):**

1. ❌ **Virus/Malware Scanning:** Out of scope (relies on browser sandboxing)
2. ❌ **Advanced Steganography:** Image reprocessing mitigates but doesn't eliminate
3. ❌ **Storage Exhaustion:** No per-user quotas yet

### Defense-in-Depth Layers

1. Rate limiting (prevents spam)
2. Authentication (only registered users)
3. Content-type header check (basic validation)
4. Magic number validation (prevents spoofing)
5. Image decode validation (ensures well-formed)
6. Image re-encode (strips metadata, sanitizes)
7. Size limits (prevents oversized files)

## Migration Plan

### Phase 1: Add Components

1. Add dependencies to `Cargo.toml`
2. Create `src/services/image_processing_service.rs`
3. Create `src/middleware/rate_limit.rs`
4. Add exports to `src/services/mod.rs` and `src/middleware/mod.rs`

### Phase 2: Update Routes

1. Modify `src/routes/upload.rs`:
   - Replace `extract_validated_image()` with `extract_multipart_file()`
   - Add processing step to upload handlers
   - Keep existing error handling
2. Update `src/routes/mod.rs` to wire rate limiting
3. Update `src/main.rs` to include rate limit state in AppState

### Phase 3: Test & Deploy

1. Run unit tests for new services
2. Run integration tests for upload flow
3. Manual testing with ExifTool verification
4. Deploy to staging environment
5. Monitor performance metrics
6. Deploy to production

### Rollback Plan

If issues arise, remove rate limit middleware layer and revert upload handlers to use existing validation. R2 uploads remain unchanged.

## Future Enhancements

### Potential Additions (Not in This Spec)

1. **Storage Quotas:** Track total uploaded bytes per user
2. **Image Optimization:** Resize/compress images for faster serving
3. **CDN Integration:** Serve images through Cloudflare CDN
4. **File Type Expansion:** Support PDFs, documents
5. **Redis Rate Limiting:** For multi-instance deployments
6. **Virus Scanning:** Integrate ClamAV or cloud scanning service
7. **Thumbnail Generation:** Auto-generate preview sizes

## Configuration

### Environment Variables

No new environment variables required. Existing R2 configuration continues to work:

- `R2_ACCOUNT_ID`
- `R2_ACCESS_KEY_ID`
- `R2_SECRET_ACCESS_KEY`
- `R2_BUCKET_NAME`
- `R2_PUBLIC_URL`

### Constants

Defined in code:

```rust
// src/routes/upload.rs
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

// src/middleware/rate_limit.rs
const RATE_LIMIT_MAX: u32 = 10;
const RATE_LIMIT_WINDOW_SECS: i64 = 3600; // 1 hour
```

## API Contract

### No Breaking Changes

All existing endpoints maintain their current request/response format:

**POST /api/upload/image**
- Request: `multipart/form-data` with `image` field
- Response: `{ "success": true, "message": "...", "data": { "url": "..." } }`

**POST /api/upload/profile-picture**
- Request: `multipart/form-data` with `image` field
- Response: `{ "success": true, "message": "...", "data": { "url": "..." } }`

**DELETE /api/upload/delete**
- Request: `{ "url": "..." }`
- Response: `{ "success": true, "message": "..." }`

### New Error Responses

- 429 Too Many Requests (new)
- 400 Bad Request with enhanced messages (magic number/type mismatch errors)

## Success Criteria

1. ✅ Magic number validation prevents file type spoofing
2. ✅ Uploaded images contain no EXIF metadata (verified with ExifTool)
3. ✅ Rate limiting blocks 11th upload within 1 hour window
4. ✅ Upload latency remains under 2 seconds for 5MB images
5. ✅ All existing upload tests pass
6. ✅ No breaking changes to API contracts

## References

- [Axum Multipart Documentation](https://docs.rs/axum/latest/axum/extract/struct.Multipart.html)
- [infer crate - Magic Number Detection](https://docs.rs/infer/latest/infer/)
- [image crate - Image Processing](https://docs.rs/image/latest/image/)
- [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
- [Cloudflare R2 Documentation](https://developers.cloudflare.com/r2/)
