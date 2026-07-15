use axum::{extract::{Path, Query, State}, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::pagination_dto::{PaginationParams, PaginationMeta},
    error::Result,
    models::user::SafeUser,
    repositories::user_repository,
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8))]
    pub password: String,
    
    #[validate(length(min = 1))]
    pub role: String, // "MEMBER", "ADMIN", or "MANAGEMENT"
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRoleDto {
    #[validate(length(min = 1))]
    pub role: String, // "MEMBER", "ADMIN", or "MANAGEMENT"
}

// Admin only - create new user
pub async fn create_user(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Json(payload): Json<CreateUserDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Validate role
    let role = match payload.role.as_str() {
        "MEMBER" | "ADMIN" | "MANAGEMENT" | "EVENT_ORGANIZER" | "BLOG_EDITOR" | "RESOURCE_MANAGER" | "TEAM_LEAD" => payload.role.as_str(),
        _ => return Err(crate::error::ApiError::BadRequest(
            "Invalid role. Must be MEMBER, EVENT_ORGANIZER, BLOG_EDITOR, RESOURCE_MANAGER, TEAM_LEAD, MANAGEMENT, or ADMIN".to_string()
        )),
    };
    
    // Check if email already exists
    if user_repository::find_by_email(&state.db, &payload.email).await?.is_some() {
        return Err(crate::error::ApiError::Conflict(
            "Email already registered".to_string()
        ));
    }
    
    // Hash password
    let password_hash = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
        .map_err(|_| crate::error::ApiError::InternalServerError)?;
    
    // Create user with specified role
    let user = sqlx::query(
        r#"
        INSERT INTO users (name, email, password, role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, email, role, profile_picture, created_at, updated_at
        "#
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(role)
    .fetch_one(&state.db)
    .await?;
    
    let safe_user = SafeUser {
        id: user.get("id"),
        name: user.get("name"),
        email: user.get("email"),
        role: user.get("role"),
        profile_picture: user.get("profile_picture"),
        created_at: user.get("created_at"),
        updated_at: user.get("updated_at"),
    };
    
    Ok(Json(json!({
        "success": true,
        "message": format!("User created successfully with {} role", role),
        "data": safe_user
    })))
}

// Admin only - list all users with their roles (with pagination)
pub async fn list_users(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated users
    let users = sqlx::query_as::<_, SafeUser>(
        r#"
        SELECT id, name, email, role, profile_picture, created_at, updated_at
        FROM users
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
        "data": users,
        "pagination": pagination
    })))
}

// Admin only - update any user's role
pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(admin): Extension<SafeUser>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRoleDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Validate role
    let role = match payload.role.as_str() {
        "MEMBER" | "ADMIN" | "MANAGEMENT" | "EVENT_ORGANIZER" | "BLOG_EDITOR" | "RESOURCE_MANAGER" | "TEAM_LEAD" => payload.role.as_str(),
        _ => return Err(crate::error::ApiError::BadRequest(
            "Invalid role. Must be MEMBER, EVENT_ORGANIZER, BLOG_EDITOR, RESOURCE_MANAGER, TEAM_LEAD, MANAGEMENT, or ADMIN".to_string()
        )),
    };
    
    // Prevent admin from changing their own role
    if user_id == admin.id {
        return Err(crate::error::ApiError::BadRequest(
            "You cannot change your own role".to_string()
        ));
    }
    
    // Update user role
    sqlx::query(
        r#"
        UPDATE users 
        SET role = $1, updated_at = NOW() 
        WHERE id = $2
        "#
    )
    .bind(role)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": format!("User role updated to {}", role)
    })))
}

// Admin only - get user by ID
pub async fn get_user(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let user = sqlx::query_as::<_, SafeUser>(
        r#"
        SELECT id, name, email, role, profile_picture, created_at, updated_at
        FROM users
        WHERE id = $1
        "#
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::ApiError::NotFound("User not found".to_string()))?;
    
    Ok(Json(json!({
        "success": true,
        "data": user
    })))
}

// Admin only - delete user
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(admin): Extension<SafeUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Prevent admin from deleting themselves
    if user_id == admin.id {
        return Err(crate::error::ApiError::BadRequest(
            "You cannot delete your own account".to_string()
        ));
    }
    
    // Delete user
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("User not found".to_string()));
    }
    
    Ok(Json(json!({
        "success": true,
        "message": "User deleted successfully"
    })))
}
