import re

with open('src/routes/notifications.rs', 'r') as f:
    content = f.read()

content = content.replace(
"""// Get all announcements (public)
pub async fn get_announcements(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let announcements = announcement_repository::find_all(&state.db).await?;
    
    Ok(Json(json!({
        "success": true,
        "data": announcements
    })))
}""",
"""// Get all announcements (public)
pub async fn get_announcements(
    State(state): State<AppState>,
    Query(mut params): Query<PaginationParams>,
) -> Result<impl axum::response::IntoResponse> {
    params.validate();
    
    let total = announcement_repository::count_all(&state.db).await?;
    let announcements = announcement_repository::find_all_paginated(&state.db, params.limit, params.offset()).await?;
    let pagination = PaginationMeta::new(params.page, params.limit, total);
    
    Ok(crate::dto::response_dto::ApiResponse::success_paginated(announcements, pagination))
}""")

with open('src/routes/notifications.rs', 'w') as f:
    f.write(content)
