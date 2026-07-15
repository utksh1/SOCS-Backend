use axum::{extract::State, Json};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::AppState;

/// Health check endpoint that touches the database to keep it active
/// This endpoint performs a simple database query to ensure both the service
/// and database remain active on Render's free tier (30-day inactivity limit)
pub async fn health_check_with_db(State(state): State<AppState>) -> Json<Value> {
    // Perform a simple database query to keep it active
    match sqlx::query("SELECT 1 as health_check")
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => Json(json!({
            "status": "healthy",
            "database": "connected",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
        Err(e) => Json(json!({
            "status": "unhealthy",
            "database": "disconnected",
            "error": e.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Simple health check without database access
pub async fn health_check() -> &'static str {
    "OK"
}
