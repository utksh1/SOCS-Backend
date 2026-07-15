use axum::http::StatusCode;
use sqlx::PgPool;

use crate::helpers::{TestApp, fixtures::UserFixture};

#[sqlx::test]
async fn test_auth_middleware_valid_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_, token) = UserFixture::create_member(&pool).await;

    // Use /api/auth/me which requires auth_middleware
    let res = app.get("/api/auth/me", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_auth_middleware_missing_token(pool: PgPool) {
    let app = TestApp::new(pool).await;
    
    // No token provided
    let res = app.get("/api/auth/me", None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_auth_middleware_invalid_token(pool: PgPool) {
    let app = TestApp::new(pool).await;
    
    // Invalid token provided
    let res = app.get("/api/auth/me", Some("invalid.jwt.token")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_optional_auth_middleware(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_, token) = UserFixture::create_member(&pool).await;

    // Use /api/projects which uses optional_auth_middleware
    // With valid token
    let res_with_auth = app.get("/api/projects", Some(&token)).await;
    assert_eq!(res_with_auth.status(), StatusCode::OK);
    
    // Without token - should still succeed (200 OK)
    let res_no_auth = app.get("/api/projects", None).await;
    assert_eq!(res_no_auth.status(), StatusCode::OK);
    
    // Note: To test the actual difference, we would need to check if the user is attached to the request,
    // which usually means testing an endpoint that behaves differently based on auth presence.
}

#[sqlx::test]
async fn test_require_toplead_success(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_, token) = UserFixture::create_toplead(&pool).await;

    // Use a route protected by require_toplead (e.g. /api/auth/register)
    // We send an empty body; it might fail with 422 Unprocessable Entity due to missing payload,
    // but it should NOT fail with 401 Unauthorized or 403 Forbidden.
    let payload = serde_json::json!({});
    let res = app.post_json("/api/auth/register", &payload, Some(&token)).await;
    
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(res.status(), StatusCode::FORBIDDEN);
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY); // Reaches the handler!
}

#[sqlx::test]
async fn test_require_toplead_forbidden_for_lower_roles(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_, mentor_token) = UserFixture::create_mentor(&pool).await;
    let (_, member_token) = UserFixture::create_member(&pool).await;

    let payload = serde_json::json!({});
    
    // Mentor should be forbidden
    let res_mentor = app.post_json("/api/auth/register", &payload, Some(&mentor_token)).await;
    assert_eq!(res_mentor.status(), StatusCode::FORBIDDEN);
    
    // Member should be forbidden
    let res_member = app.post_json("/api/auth/register", &payload, Some(&member_token)).await;
    assert_eq!(res_member.status(), StatusCode::FORBIDDEN);
}
