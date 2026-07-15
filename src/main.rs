use std::{env, net::SocketAddr};

use socs_backend::{
    build_router,
    config::{database, env::Config},
    services::cleanup_service,
    tasks,
    telemetry,
    AppState,
};

#[tokio::main]
async fn main() {
    let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    telemetry::init_telemetry(&environment);

    let config = Config::from_env().expect("Failed to load configuration");
    tracing::info!("Starting SOCS Backend (Rust)");

    let db = database::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");
    tracing::info!("Database connection established");

    let redis = if !config.redis_url.is_empty() {
        let redis_client = redis::Client::open(config.redis_url.as_str())
            .expect("Failed to create Redis client");
        let connection = redis_client
            .get_connection_manager()
            .await
            .expect("Failed to create Redis connection manager");
        tracing::info!("Redis connection established");
        Some(connection)
    } else {
        tracing::warn!("Redis URL is empty; Redis was not connected");
        None
    };

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run migrations");
    tracing::info!("Migrations completed successfully");

    let host = config.host.clone();
    let port = config.port;
    let state = AppState { db, config, redis };

    let verification_cleanup_pool = state.db.clone();
    tokio::spawn(async move {
        tasks::cleanup::start_cleanup_task(verification_cleanup_pool).await;
    });

    let soft_delete_cleanup_pool = state.db.clone();
    tokio::spawn(async move {
        if let Err(error) = cleanup_service::start_cleanup_scheduler(soft_delete_cleanup_pool).await {
            tracing::error!(%error, "Failed to start soft-delete cleanup scheduler");
        }
    });

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    telemetry::log_startup_info(&host, port);
    axum::serve(
        listener,
        build_router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");
}
