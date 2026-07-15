use axum::Router;
use sqlx::PgPool;

/// Build the Axum app with test configuration
pub async fn build_app(pool: PgPool) -> Router {
    // Load test configuration
    let config = socs_backend::config::env::Config {
        database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        jwt_secret: "test_jwt_secret_key_12345".to_string(),
        jwt_expires_in: 3600,
        redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        host: "127.0.0.1".to_string(),
        port: 8080,
        cors_origin: "http://localhost:3000".to_string(),
        frontend_url: "http://localhost:3000".to_string(),
    };
    
    // Create a Redis connection manager
    let redis_client = redis::Client::open(config.redis_url.as_str())
        .expect("Failed to create Redis client");
    let redis = redis_client
        .get_connection_manager()
        .await
        .expect("Failed to create Redis connection manager");
    
    let state = socs_backend::AppState {
        db: pool,
        config,
        redis,
    };
    
    // Build the router using the library function
    socs_backend::build_router(state)
}
