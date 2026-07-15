use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::AppState;

/// Health check endpoint that writes to the database to keep it active
/// This endpoint performs a write operation to ensure the database
/// remains active on Render's free tier (30-day inactivity limit)
/// 
/// Creates a keepalive table if it doesn't exist and updates a timestamp record
pub async fn health_check_with_db(State(state): State<AppState>) -> Json<Value> {
    // Create keepalive table if it doesn't exist
    let create_table = sqlx::query(
        "CREATE TABLE IF NOT EXISTS system_keepalive (
            id INTEGER PRIMARY KEY DEFAULT 1,
            last_ping TIMESTAMPTZ NOT NULL,
            ping_count BIGINT NOT NULL DEFAULT 0,
            CONSTRAINT single_row CHECK (id = 1)
        )"
    )
    .execute(&state.db)
    .await;

    if let Err(e) = create_table {
        return Json(json!({
            "status": "unhealthy",
            "database": "table_creation_failed",
            "error": e.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));
    }

    // Upsert keepalive record - this performs a WRITE operation
    match sqlx::query(
        "INSERT INTO system_keepalive (id, last_ping, ping_count)
         VALUES (1, NOW(), 1)
         ON CONFLICT (id)
         DO UPDATE SET
            last_ping = NOW(),
            ping_count = system_keepalive.ping_count + 1"
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => {
            // Fetch the updated record to show activity
            match sqlx::query_as::<_, (chrono::DateTime<chrono::Utc>, i64)>(
                "SELECT last_ping, ping_count FROM system_keepalive WHERE id = 1"
            )
            .fetch_one(&state.db)
            .await
            {
                Ok((last_ping, ping_count)) => Json(json!({
                    "status": "healthy",
                    "database": "active",
                    "last_ping": last_ping.to_rfc3339(),
                    "ping_count": ping_count,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
                Err(e) => Json(json!({
                    "status": "warning",
                    "database": "write_succeeded_read_failed",
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            }
        },
        Err(e) => Json(json!({
            "status": "unhealthy",
            "database": "write_failed",
            "error": e.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Simple health check without database access
pub async fn health_check() -> &'static str {
    "OK"
}
