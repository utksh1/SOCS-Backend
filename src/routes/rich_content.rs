use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::json;
use crate::dto::response_dto::ApiResponse;
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::rich_content_dto::*,
    error::Result,
    models::{rich_content::*, user::SafeUser},
    AppState,
};

// ============================================================================
// PROJECT FEATURES
// ============================================================================

pub async fn list_project_features(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let features = sqlx::query_as::<_, ProjectFeature>(
        "SELECT * FROM project_features WHERE project_id = $1 ORDER BY display_order ASC"
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await?;

    Ok(ApiResponse::success(features))
}

pub async fn create_project_feature(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateProjectFeatureDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    
    // Check permission

    let feature = sqlx::query_as::<_, ProjectFeature>(
        r#"
        INSERT INTO project_features (project_id, title, description, display_order)
        VALUES ($1, $2, $3, COALESCE($4, (SELECT COALESCE(MAX(display_order), 0) + 1 FROM project_features WHERE project_id = $1)))
        RETURNING *
        "#
    )
    .bind(project_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": feature
    }))))
}

pub async fn update_project_feature(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((project_id, feature_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateProjectFeatureDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let feature = sqlx::query_as::<_, ProjectFeature>(
        r#"
        UPDATE project_features
        SET title = COALESCE($1, title),
            description = COALESCE($2, description),
            display_order = COALESCE($3, display_order),
            updated_at = NOW()
        WHERE id = $4 AND project_id = $5
        RETURNING *
        "#
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .bind(feature_id)
    .bind(project_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ApiResponse::success(feature))
}

pub async fn delete_project_feature(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((project_id, feature_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse> {

    let result = sqlx::query(
        "DELETE FROM project_features WHERE id = $1 AND project_id = $2"
    )
    .bind(feature_id)
    .bind(project_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("Feature not found".to_string()));
    }

    Ok(ApiResponse::success_with_message("Feature deleted", json!({})))
}

pub async fn reorder_project_features(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<ReorderItemsDto>,
) -> Result<impl axum::response::IntoResponse> {

    for item in &payload.items {
        let id = Uuid::parse_str(&item.id)
            .map_err(|_| crate::error::ApiError::BadRequest("Invalid UUID".to_string()))?;
        
        sqlx::query(
            "UPDATE project_features SET display_order = $1 WHERE id = $2 AND project_id = $3"
        )
        .bind(item.display_order)
        .bind(id)
        .bind(project_id)
        .execute(&state.db)
        .await?;
    }

    Ok(ApiResponse::success_with_message("Features reordered", json!({})))
}

// ============================================================================
// PROJECT CONTRIBUTORS
// ============================================================================

pub async fn list_project_contributors(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let contributors = sqlx::query_as::<_, ProjectContributorWithMember>(
        r#"
        SELECT 
            pc.id, pc.project_id, pc.team_member_id, pc.role, pc.joined_at,
            t.name as member_name, t.avatar_url as member_avatar, t.slug as member_slug
        FROM project_contributors pc
        JOIN team t ON pc.team_member_id = t.id
        WHERE pc.project_id = $1
        ORDER BY pc.joined_at DESC
        "#
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await?;

    Ok(ApiResponse::success(contributors))
}

pub async fn add_project_contributor(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateProjectContributorDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let team_member_id = Uuid::parse_str(&payload.team_member_id)
        .map_err(|_| crate::error::ApiError::BadRequest("Invalid team member ID".to_string()))?;

    let contributor = sqlx::query_as::<_, ProjectContributor>(
        r#"
        INSERT INTO project_contributors (project_id, team_member_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#
    )
    .bind(project_id)
    .bind(team_member_id)
    .bind(&payload.role)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": contributor
    }))))
}

pub async fn remove_project_contributor(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((project_id, contributor_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse> {

    let result = sqlx::query(
        "DELETE FROM project_contributors WHERE id = $1 AND project_id = $2"
    )
    .bind(contributor_id)
    .bind(project_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("Contributor not found".to_string()));
    }

    Ok(ApiResponse::success_with_message("Contributor removed", json!({})))
}

// ============================================================================
// EVENT TIMELINE
// ============================================================================

pub async fn list_event_timeline(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let timeline = sqlx::query_as::<_, EventTimelineItem>(
        "SELECT * FROM event_timeline_items WHERE event_id = $1 ORDER BY display_order ASC"
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    Ok(ApiResponse::success(timeline))
}

pub async fn create_event_timeline_item(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CreateEventTimelineDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let item = sqlx::query_as::<_, EventTimelineItem>(
        r#"
        INSERT INTO event_timeline_items (event_id, time, title, description, display_order)
        VALUES ($1, $2, $3, $4, COALESCE($5, (SELECT COALESCE(MAX(display_order), 0) + 1 FROM event_timeline_items WHERE event_id = $1)))
        RETURNING *
        "#
    )
    .bind(event_id)
    .bind(&payload.time)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": item
    }))))
}

pub async fn update_event_timeline_item(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((event_id, item_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateEventTimelineDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let item = sqlx::query_as::<_, EventTimelineItem>(
        r#"
        UPDATE event_timeline_items
        SET time = COALESCE($1, time),
            title = COALESCE($2, title),
            description = COALESCE($3, description),
            display_order = COALESCE($4, display_order),
            updated_at = NOW()
        WHERE id = $5 AND event_id = $6
        RETURNING *
        "#
    )
    .bind(&payload.time)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .bind(item_id)
    .bind(event_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ApiResponse::success(item))
}

pub async fn delete_event_timeline_item(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((event_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse> {

    let result = sqlx::query(
        "DELETE FROM event_timeline_items WHERE id = $1 AND event_id = $2"
    )
    .bind(item_id)
    .bind(event_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("Timeline item not found".to_string()));
    }

    Ok(ApiResponse::success_with_message("Timeline item deleted", json!({})))
}

pub async fn reorder_event_timeline(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<ReorderItemsDto>,
) -> Result<impl axum::response::IntoResponse> {

    for item in &payload.items {
        let id = Uuid::parse_str(&item.id)
            .map_err(|_| crate::error::ApiError::BadRequest("Invalid UUID".to_string()))?;
        
        sqlx::query(
            "UPDATE event_timeline_items SET display_order = $1 WHERE id = $2 AND event_id = $3"
        )
        .bind(item.display_order)
        .bind(id)
        .bind(event_id)
        .execute(&state.db)
        .await?;
    }

    Ok(ApiResponse::success_with_message("Timeline reordered", json!({})))
}

// ============================================================================
// EVENT PREREQUISITES
// ============================================================================

pub async fn list_event_prerequisites(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let prerequisites = sqlx::query_as::<_, EventPrerequisite>(
        "SELECT * FROM event_prerequisites WHERE event_id = $1 ORDER BY display_order ASC"
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    Ok(ApiResponse::success(prerequisites))
}

pub async fn create_event_prerequisite(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CreateEventPrerequisiteDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let prerequisite = sqlx::query_as::<_, EventPrerequisite>(
        r#"
        INSERT INTO event_prerequisites (event_id, title, description, display_order)
        VALUES ($1, $2, $3, COALESCE($4, (SELECT COALESCE(MAX(display_order), 0) + 1 FROM event_prerequisites WHERE event_id = $1)))
        RETURNING *
        "#
    )
    .bind(event_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": prerequisite
    }))))
}

pub async fn update_event_prerequisite(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((event_id, prereq_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateEventPrerequisiteDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let prerequisite = sqlx::query_as::<_, EventPrerequisite>(
        r#"
        UPDATE event_prerequisites
        SET title = COALESCE($1, title),
            description = COALESCE($2, description),
            display_order = COALESCE($3, display_order),
            updated_at = NOW()
        WHERE id = $4 AND event_id = $5
        RETURNING *
        "#
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.display_order)
    .bind(prereq_id)
    .bind(event_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ApiResponse::success(prerequisite))
}

pub async fn delete_event_prerequisite(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((event_id, prereq_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse> {

    let result = sqlx::query(
        "DELETE FROM event_prerequisites WHERE id = $1 AND event_id = $2"
    )
    .bind(prereq_id)
    .bind(event_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("Prerequisite not found".to_string()));
    }

    Ok(ApiResponse::success_with_message("Prerequisite deleted", json!({})))
}

pub async fn reorder_event_prerequisites(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<ReorderItemsDto>,
) -> Result<impl axum::response::IntoResponse> {

    for item in &payload.items {
        let id = Uuid::parse_str(&item.id)
            .map_err(|_| crate::error::ApiError::BadRequest("Invalid UUID".to_string()))?;
        
        sqlx::query(
            "UPDATE event_prerequisites SET display_order = $1 WHERE id = $2 AND event_id = $3"
        )
        .bind(item.display_order)
        .bind(id)
        .bind(event_id)
        .execute(&state.db)
        .await?;
    }

    Ok(ApiResponse::success_with_message("Prerequisites reordered", json!({})))
}

// ============================================================================
// TEAM CONTRIBUTIONS
// ============================================================================

pub async fn list_team_contributions(
    State(state): State<AppState>,
    Path(member_id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let contributions = sqlx::query_as::<_, TeamContribution>(
        "SELECT * FROM team_contributions WHERE team_member_id = $1 ORDER BY contribution_date DESC"
    )
    .bind(member_id)
    .fetch_all(&state.db)
    .await?;

    Ok(ApiResponse::success(contributions))
}

pub async fn create_team_contribution(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path(member_id): Path<Uuid>,
    Json(payload): Json<CreateTeamContributionDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let contribution_date = chrono::NaiveDate::parse_from_str(&payload.contribution_date, "%Y-%m-%d")
        .map_err(|_| crate::error::ApiError::BadRequest("Invalid date format, use YYYY-MM-DD".to_string()))?;

    let contribution = sqlx::query_as::<_, TeamContribution>(
        r#"
        INSERT INTO team_contributions (team_member_id, contribution_type, title, description, url, contribution_date)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(member_id)
    .bind(&payload.contribution_type)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.url)
    .bind(contribution_date)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({
        "success": true,
        "data": contribution
    }))))
}

pub async fn update_team_contribution(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((member_id, contribution_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateTeamContributionDto>,
) -> Result<impl axum::response::IntoResponse> {
    payload.validate()?;
    

    let contribution_date = if let Some(date_str) = &payload.contribution_date {
        Some(chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| crate::error::ApiError::BadRequest("Invalid date format, use YYYY-MM-DD".to_string()))?)
    } else {
        None
    };

    let contribution = sqlx::query_as::<_, TeamContribution>(
        r#"
        UPDATE team_contributions
        SET contribution_type = COALESCE($1, contribution_type),
            title = COALESCE($2, title),
            description = COALESCE($3, description),
            url = COALESCE($4, url),
            contribution_date = COALESCE($5, contribution_date),
            updated_at = NOW()
        WHERE id = $6 AND team_member_id = $7
        RETURNING *
        "#
    )
    .bind(&payload.contribution_type)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.url)
    .bind(contribution_date)
    .bind(contribution_id)
    .bind(member_id)
    .fetch_one(&state.db)
    .await?;

    Ok(ApiResponse::success(contribution))
}

pub async fn delete_team_contribution(
    State(state): State<AppState>,
    Extension(_user): Extension<SafeUser>,
    Path((member_id, contribution_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse> {

    let result = sqlx::query(
        "DELETE FROM team_contributions WHERE id = $1 AND team_member_id = $2"
    )
    .bind(contribution_id)
    .bind(member_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound("Contribution not found".to_string()));
    }

    Ok(ApiResponse::success_with_message("Contribution deleted", json!({})))
}
