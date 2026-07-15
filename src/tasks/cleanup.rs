use sqlx::PgPool;
use tokio::time::{interval, Duration};

use crate::services::verification_service;

/// Run daily cleanup of expired tokens and rate limits
pub async fn start_cleanup_task(pool: PgPool) {
    let mut interval = interval(Duration::from_secs(86400)); // 24 hours

    loop {
        interval.tick().await;

        tracing::info!("Starting scheduled cleanup task");

        match verification_service::cleanup_expired_data(&pool).await {
            Ok(_) => {
                tracing::info!("Scheduled cleanup completed successfully");
            }
            Err(e) => {
                tracing::error!(error = ?e, "Scheduled cleanup failed");
            }
        }
    }
}
