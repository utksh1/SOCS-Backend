use axum::{extract::{Path, Query, State}, http::StatusCode, Extension, Json};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;
use crate::{
    dto::{
        team_dto::CreateTeamMemberDto,
        pagination_dto::{PaginationParams, PaginationMeta},
    },
    error::Result,
    models::user::SafeUser,
    repositories::team_repository,
    utils::slugify::slugify,
    AppState
};

pub async fn list_team(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>> {
    params.validate();
    
    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team")
        .fetch_one(&state.db)
        .await?;
    
    // Get paginated team members
    let team: Vec<crate::models::team::TeamMember> = sqlx::query_as(
        r#"
        SELECT id, slug, name, role, bio, skills, tier, github, linkedin,
               avatar_url, created_by as added_by, created_at, updated_at
        FROM team
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
        "data": team,
        "pagination": pagination
    })))
}

pub async fn get_team_member(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    let member = team_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Team member not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": member})))
}

pub async fn get_team_member_by_slug(State(state): State<AppState>, Path(slug): Path<String>) -> Result<Json<serde_json::Value>> {
    let member = team_repository::find_by_slug(&state.db, &slug).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Team member not found".to_string()))?;
    Ok(Json(json!({"success": true, "data": member})))
}

pub async fn create_team_member(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Json(payload): Json<CreateTeamMemberDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    payload.validate()?;
    
    // Get the creator's tier from team table
    let creator_tier: Option<crate::models::team::MemberTier> = sqlx::query_scalar(
        "SELECT tier FROM team WHERE created_by = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    
    // Check if user has permission to create members
    // Only TopLead/Mentor can create members
    let has_permission = creator_tier.as_ref().map(|t| t.can_create_delete_members()).unwrap_or(false);
    
    if !has_permission {
        return Err(crate::error::ApiError::Forbidden(
            "Only TopLead or Mentor tier members can create team members".to_string()
        ));
    }
    
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.name));
    
    let member = team_repository::create(
        &state.db, &slug, &payload.name, &payload.role, payload.bio.as_deref(), &payload.skills,
        payload.tier.unwrap_or(crate::models::team::MemberTier::Member),
        payload.github.as_deref(), payload.linkedin.as_deref(), payload.avatar_url.as_deref(), Some(user.id),
    ).await?;
    
    Ok((StatusCode::CREATED, Json(json!({"success": true, "message": "Team member created", "data": member}))))
}

pub async fn update_team_member(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateTeamMemberDto>,
) -> Result<Json<serde_json::Value>> {
    payload.validate()?;
    
    // Check if member exists and get their current tier
    let existing = team_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Team member not found".to_string()))?;
    
    // Get the updater's tier from team table
    let updater_tier: Option<crate::models::team::MemberTier> = sqlx::query_scalar(
        "SELECT tier FROM team WHERE created_by = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    
    // TopLead/Mentor/Core can update members below their tier
    let has_permission = updater_tier.as_ref().map(|t| t.can_manage(&existing.tier)).unwrap_or(false);
    
    if !has_permission {
        return Err(crate::error::ApiError::Forbidden(
            "You don't have permission to update this team member".to_string()
        ));
    }
    
    // Prevent updating tier to a level higher than updater's tier
    if let Some(new_tier) = &payload.tier {
        if let Some(ref updater_t) = updater_tier {
            if updater_t.level() < new_tier.level() {
                return Err(crate::error::ApiError::Forbidden(
                    "You cannot assign a tier higher than your own".to_string()
                ));
            }
        }
    }
    
    let slug = payload.slug.clone().unwrap_or_else(|| slugify(&payload.name));
    
    // Update team member
    let member: crate::models::team::TeamMember = sqlx::query_as(
        r#"
        UPDATE team
        SET slug = $1, name = $2, role = $3, bio = $4, skills = $5, tier = $6,
            github = $7, linkedin = $8, avatar_url = $9, updated_at = NOW()
        WHERE id = $10
        RETURNING id, slug, name, role, bio, skills, tier, github, linkedin,
                  avatar_url, created_by, created_at, updated_at
        "#
    )
    .bind(&slug)
    .bind(&payload.name)
    .bind(&payload.role)
    .bind(&payload.bio)
    .bind(&payload.skills)
    .bind(payload.tier.unwrap_or(existing.tier))
    .bind(&payload.github)
    .bind(&payload.linkedin)
    .bind(&payload.avatar_url)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(json!({"success": true, "message": "Team member updated", "data": member})))
}

pub async fn delete_team_member(
    State(state): State<AppState>,
    Extension(user): Extension<SafeUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Check if member exists and get their tier
    let existing = team_repository::find_by_id(&state.db, id).await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Team member not found".to_string()))?;
    
    // Get the deleter's tier from team table
    let deleter_tier: Option<crate::models::team::MemberTier> = sqlx::query_scalar(
        "SELECT tier FROM team WHERE created_by = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    
    // Only TopLead/Mentor can delete members
    let has_permission = deleter_tier.as_ref().map(|t| t.can_create_delete_members()).unwrap_or(false);
    
    if !has_permission {
        return Err(crate::error::ApiError::Forbidden(
            "Only TopLead or Mentor tier members can delete team members".to_string()
        ));
    }
    
    let deleted = team_repository::delete(&state.db, id).await?;
    if !deleted {
        return Err(crate::error::ApiError::NotFound("Team member not found".to_string()));
    }
    Ok(Json(json!({"success": true, "message": "Team member deleted"})))
}
