use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize telemetry with structured logging
/// Supports both pretty console output for development and JSON for production
pub fn init_telemetry(env: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("socs_backend=debug,tower_http=debug,sqlx=debug"));

    let is_production = env == "production";

    if is_production {
        // JSON structured logging for production (easier to parse by log aggregators)
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json().with_target(true).with_level(true))
            .init();
    } else {
        // Pretty console output for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
            )
            .init();
    }
}

/// Log application startup information
pub fn log_startup_info(host: &str, port: u16) {
    tracing::info!(
        host = %host,
        port = port,
        "🚀 SOCS Backend starting"
    );
    tracing::info!(
        api_url = %format!("http://{}:{}/api", host, port),
        health_url = %format!("http://{}:{}/health", host, port),
        "Server endpoints"
    );
}
