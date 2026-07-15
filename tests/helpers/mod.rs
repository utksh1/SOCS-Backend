use axum::{
    body::Body,
    http::{self, Request, Method},
    Router,
};
use serde::Serialize;
use serde_json::Value;
use socs_backend::{build_router, AppState};
use sqlx::PgPool;
use std::env;
use tower::ServiceExt;

pub mod fixtures;

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    pub async fn new(pool: PgPool) -> Self {
        // Mock redis or provide a real one for tests
        // But for these tests, we can just use the provided pool
        // We need a dummy Config and Redis connection for AppState
        // The real AppState requires a redis::aio::ConnectionManager
        
        dotenvy::from_filename(".env.test").ok();
        
        let config = socs_backend::config::env::Config {
            host: "127.0.0.1".into(),
            port: 8080,
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/socs_test".into()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            rate_limit_enabled: false,
            jwt_secret: "test_secret_key_12345".into(),
            jwt_expires_in: 3600,
            cors_origin: "http://localhost:3000".into(),
            frontend_url: "http://localhost:3000".into(),
        };

        let state = AppState {
            db: pool.clone(),
            config,
            redis: None,
        };

        let router = build_router(state);

        Self { router }
    }

    pub async fn request(&self, method: Method, uri: &str, body: Option<Body>, token: Option<&str>) -> http::Response<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, "application/json");
            
        if let Some(t) = token {
            builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {}", t));
        }
        
        let req = builder.body(body.unwrap_or_else(Body::empty)).unwrap();
        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> http::Response<Body> {
        self.request(Method::GET, uri, None, token).await
    }

    pub async fn delete(&self, uri: &str, token: Option<&str>) -> http::Response<Body> {
        self.request(Method::DELETE, uri, None, token).await
    }

    pub async fn post_json<T: Serialize>(&self, uri: &str, payload: &T, token: Option<&str>) -> http::Response<Body> {
        let json = serde_json::to_vec(payload).unwrap();
        self.request(Method::POST, uri, Some(Body::from(json)), token).await
    }

    pub async fn put_json<T: Serialize>(&self, uri: &str, payload: &T, token: Option<&str>) -> http::Response<Body> {
        let json = serde_json::to_vec(payload).unwrap();
        self.request(Method::PUT, uri, Some(Body::from(json)), token).await
    }
    
    pub async fn patch_json<T: Serialize>(&self, uri: &str, payload: &T, token: Option<&str>) -> http::Response<Body> {
        let json = serde_json::to_vec(payload).unwrap();
        self.request(Method::PATCH, uri, Some(Body::from(json)), token).await
    }
}

/// Build the router used by the legacy endpoint tests.
pub async fn build_app(pool: PgPool) -> Router {
    TestApp::new(pool).await.router
}

pub async fn read_body_json(response: http::Response<Body>) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}
