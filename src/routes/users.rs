use axum::{extract::{Path, Query, State}, Extension, Json};
use serde::Deserialize;
use serde_json::json;
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
    
    pub roles: Vec<crate::models::user::UserRole>, // Multiple roles: TOPLEAD, MENTOR, CORE, LEAD, MEMBER
    
    pub position: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub github: Option<String>,
    pub linkedin: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserDto {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    
    pub roles: Option<Vec<crate::models::user::UserRole>>,
    
    pub position: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub github: Option<String>,
    pub linkedin: Option<String>,
}

// TopLead/Mentor only - create new user
pub async fn create_user(
    State(state): State<AppState>,
    Extension(creator): Extension<SafeUser>,
    Json(payload): Json<CreateUserDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Check if creator has permission to create users
    crate::middleware::auth::can_create_delete_users(&creator)?;
    
    // Ensure user has at least Member role
    let mut roles = payload.roles;
    if !roles.contains(&crate::models::user::UserRole::Member) {
        roles.push(crate::models::user::UserRole::Member);
    }
    
    // Check if creator can assign all requested roles
    crate::middleware::auth::can_assign_roles(&creator, &roles)?;
    
    // Check if email already exists
    if user_repository::find_by_email(&state.db, &payload.email).await?.is_some() {
        return Err(crate::error::ApiError::Conflict(
            "Email already registered".to_string()
        ));
    }
    
    // Hash password
    let password_hash = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
        .map_err(|_| crate::error::ApiError::InternalServerError)?;
    
    // Generate slug from name
    let slug = crate::utils::slugify::slugify(&payload.name);
    
    // Create user with specified roles and profile
    let user = sqlx::query_as::<_, SafeUser>(
        r#"
        INSERT INTO users (name, email, password, roles, slug, position, bio, skills, github, linkedin)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at
        "#
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(&roles)
    .bind(&slug)
    .bind(&payload.position)
    .bind(&payload.bio)
    .bind(&payload.skills)
    .bind(&payload.github)
    .bind(&payload.linkedin)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(json!({
        "success": true,
        "message": format!("User created successfully with roles: {:?}", roles),
        "data": user
    })))
}

// Public - list all users (team directory)
pub async fn list_users(
    State(state): State<AppState>,
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
        SELECT id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at
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

// TopLead/Mentor/Core - update user profile and role
pub async fn update_user(
    State(state): State<AppState>,
    Extension(updater): Extension<SafeUser>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Get target user
    let target_user = sqlx::query_as::<_, SafeUser>(
        "SELECT id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::ApiError::NotFound("User not found".to_string()))?;
    
    // Check if updater has permission to update this user
    crate::middleware::auth::can_manage_user(&updater, &target_user)?;
    
    // If updating roles, check if updater can assign all new roles
    if let Some(ref new_roles) = payload.roles {
        // Ensure Member role is included
        let mut roles = new_roles.clone();
        if !roles.contains(&crate::models::user::UserRole::Member) {
            roles.push(crate::models::user::UserRole::Member);
        }
        
        crate::middleware::auth::can_assign_roles(&updater, &roles)?;
    }
    
    // Prevent user from changing their own roles
    if user_id == updater.id && payload.roles.is_some() {
        return Err(crate::error::ApiError::BadRequest(
            "You cannot change your own roles".to_string()
        ));
    }
    
    // Build dynamic update query
    let mut updates = Vec::new();
    let mut bind_count = 1;
    
    if payload.name.is_some() {
        updates.push(format!("name = ${}", bind_count));
        bind_count += 1;
    }
    if payload.roles.is_some() {
        updates.push(format!("roles = ${}", bind_count));
        bind_count += 1;
    }
    if payload.position.is_some() {
        updates.push(format!("position = ${}", bind_count));
        bind_count += 1;
    }
    if payload.bio.is_some() {
        updates.push(format!("bio = ${}", bind_count));
        bind_count += 1;
    }
    if payload.skills.is_some() {
        updates.push(format!("skills = ${}", bind_count));
        bind_count += 1;
    }
    if payload.github.is_some() {
        updates.push(format!("github = ${}", bind_count));
        bind_count += 1;
    }
    if payload.linkedin.is_some() {
        updates.push(format!("linkedin = ${}", bind_count));
        bind_count += 1;
    }
    
    if updates.is_empty() {
        return Err(crate::error::ApiError::BadRequest("No fields to update".to_string()));
    }
    
    updates.push("updated_at = NOW()".to_string());
    
    let query_str = format!(
        "UPDATE users SET {} WHERE id = ${} RETURNING id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at",
        updates.join(", "),
        bind_count
    );
    
    let mut query = sqlx::query_as::<_, SafeUser>(&query_str);
    
    if let Some(name) = &payload.name {
        query = query.bind(name);
    }
    if let Some(roles) = &payload.roles {
        // Ensure Member is included
        let mut final_roles = roles.clone();
        if !final_roles.contains(&crate::models::user::UserRole::Member) {
            final_roles.push(crate::models::user::UserRole::Member);
        }
        query = query.bind(final_roles);
    }
    if let Some(position) = &payload.position {
        query = query.bind(position);
    }
    if let Some(bio) = &payload.bio {
        query = query.bind(bio);
    }
    if let Some(skills) = &payload.skills {
        query = query.bind(skills);
    }
    if let Some(github) = &payload.github {
        query = query.bind(github);
    }
    if let Some(linkedin) = &payload.linkedin {
        query = query.bind(linkedin);
    }
    
    let user = query.bind(user_id).fetch_one(&state.db).await?;
    
    Ok(Json(json!({
        "success": true,
        "message": "User updated successfully",
        "data": user
    })))
}

// Public - get user by ID
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let user = sqlx::query_as::<_, SafeUser>(
        r#"
        SELECT id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at
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

// Public - get user by slug
pub async fn get_user_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let user = sqlx::query_as::<_, SafeUser>(
        r#"
        SELECT id, name, email, roles, slug, position, bio, skills, github, linkedin, avatar_url, profile_picture, created_at, updated_at
        FROM users
        WHERE slug = $1
        "#
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::ApiError::NotFound("User not found".to_string()))?;
    
    Ok(Json(json!({
        "success": true,
        "data": user
    })))
}

// TopLead/Mentor - delete user
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(deleter): Extension<SafeUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if deleter has permission
    crate::middleware::auth::can_create_delete_users(&deleter)?;
    
    // Prevent user from deleting themselves
    if user_id == deleter.id {
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
