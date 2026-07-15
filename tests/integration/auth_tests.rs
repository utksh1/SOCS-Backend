use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use crate::{
    common::{make_request, response_json, assert_status},
    fixtures::{UserFixture, AuthFixture},
    helpers::build_app,
};

#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create a TopLead user to authenticate the registration request
    let toplead = UserFixture::new()
        .email("toplead@test.com")
        .name("Top Lead")
        .role(socs_backend::models::user::UserRole::TopLead)
        .insert(&pool)
        .await
        .expect("Failed to create toplead");
    
    let token = AuthFixture::generate_token_with_role(toplead.id, socs_backend::models::user::UserRole::TopLead, "test_secret_key_12345");
    
    let payload = json!({
        "name": "New User",
        "email": "newuser@test.com",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    assert_status(&response, StatusCode::CREATED);
    
    let body = response_json(response).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "User registered successfully. Please check your email to verify your account.");
    assert!(body["data"]["token"].is_string());
    assert_eq!(body["data"]["user"]["email"], "newuser@test.com");
    assert_eq!(body["data"]["user"]["name"], "New User");
}

#[sqlx::test]
async fn test_register_validation_short_name(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let toplead = UserFixture::new()
        .email("toplead@test.com")
        .role(socs_backend::models::user::UserRole::TopLead)
        .insert(&pool)
        .await
        .expect("Failed to create toplead");
    
    let token = AuthFixture::generate_token_with_role(toplead.id, socs_backend::models::user::UserRole::TopLead, "test_secret_key_12345");
    
    let payload = json!({
        "name": "A",  // Too short (min 2)
        "email": "test@test.com",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_register_validation_invalid_email(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let toplead = UserFixture::new()
        .email("toplead@test.com")
        .role(socs_backend::models::user::UserRole::TopLead)
        .insert(&pool)
        .await
        .expect("Failed to create toplead");
    
    let token = AuthFixture::generate_token_with_role(toplead.id, socs_backend::models::user::UserRole::TopLead, "test_secret_key_12345");
    
    let payload = json!({
        "name": "Test User",
        "email": "not-an-email",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_register_validation_short_password(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let toplead = UserFixture::new()
        .email("toplead@test.com")
        .role(socs_backend::models::user::UserRole::TopLead)
        .insert(&pool)
        .await
        .expect("Failed to create toplead");
    
    let token = AuthFixture::generate_token_with_role(toplead.id, socs_backend::models::user::UserRole::TopLead, "test_secret_key_12345");
    
    let payload = json!({
        "name": "Test User",
        "email": "test@test.com",
        "password": "short"  // Too short (min 8)
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_register_duplicate_email(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create existing user
    UserFixture::new()
        .email("existing@test.com")
        .insert(&pool)
        .await
        .expect("Failed to create existing user");
    
    // Create TopLead for auth
    let toplead = UserFixture::new()
        .email("toplead@test.com")
        .role(socs_backend::models::user::UserRole::TopLead)
        .insert(&pool)
        .await
        .expect("Failed to create toplead");
    
    let token = AuthFixture::generate_token_with_role(toplead.id, socs_backend::models::user::UserRole::TopLead, "test_secret_key_12345");
    
    let payload = json!({
        "name": "Another User",
        "email": "existing@test.com",  // Duplicate
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    // Should fail with conflict or bad request
    assert!(response.status() == StatusCode::CONFLICT || response.status() == StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_register_requires_toplead(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create a non-TopLead user
    let member = UserFixture::new()
        .email("member@test.com")
        .role(socs_backend::models::user::UserRole::Member)
        .insert(&pool)
        .await
        .expect("Failed to create member");
    
    let token = AuthFixture::generate_token_with_role(member.id, socs_backend::models::user::UserRole::Member, "test_secret_key_12345");
    
    let payload = json!({
        "name": "New User",
        "email": "newuser@test.com",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/register", Some(&token), Some(payload)).await;
    
    // Should fail with forbidden
    assert_status(&response, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_login_success(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create user with known password
    UserFixture::new()
        .email("user@test.com")
        .name("Test User")
        .password("password123")
        .insert(&pool)
        .await
        .expect("Failed to create user");
    
    let payload = json!({
        "email": "user@test.com",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/login", None, Some(payload)).await;
    
    assert_status(&response, StatusCode::OK);
    
    let body = response_json(response).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Login successful");
    assert!(body["data"]["token"].is_string());
    assert_eq!(body["data"]["user"]["email"], "user@test.com");
    assert_eq!(body["data"]["user"]["name"], "Test User");
}

#[sqlx::test]
async fn test_login_wrong_password(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create user with known password
    UserFixture::new()
        .email("user@test.com")
        .password("correctpassword")
        .insert(&pool)
        .await
        .expect("Failed to create user");
    
    let payload = json!({
        "email": "user@test.com",
        "password": "wrongpassword"
    });
    
    let response = make_request(app, "POST", "/api/auth/login", None, Some(payload)).await;
    
    // Should fail with unauthorized
    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_login_nonexistent_user(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let payload = json!({
        "email": "nonexistent@test.com",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/login", None, Some(payload)).await;
    
    // Should fail with unauthorized or not found
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || 
        response.status() == StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn test_login_validation_invalid_email(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let payload = json!({
        "email": "not-an-email",
        "password": "password123"
    });
    
    let response = make_request(app, "POST", "/api/auth/login", None, Some(payload)).await;
    
    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_get_me_with_valid_token(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create user
    let user = UserFixture::new()
        .email("user@test.com")
        .name("Test User")
        .insert(&pool)
        .await
        .expect("Failed to create user");
    
    let token = AuthFixture::generate_token(user.id, "test_secret_key_12345");
    
    let response = make_request(app, "GET", "/api/auth/me", Some(&token), None).await;
    
    assert_status(&response, StatusCode::OK);
    
    let body = response_json(response).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["id"], user.id.to_string());
    assert_eq!(body["data"]["email"], "user@test.com");
    assert_eq!(body["data"]["name"], "Test User");
}

#[sqlx::test]
async fn test_get_me_with_expired_token(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create user
    let user = UserFixture::new()
        .email("user@test.com")
        .insert(&pool)
        .await
        .expect("Failed to create user");
    
    let expired_token = AuthFixture::generate_expired_token(user.id, "test_secret_key_12345");
    
    let response = make_request(app, "GET", "/api/auth/me", Some(&expired_token), None).await;
    
    // Should fail with unauthorized
    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_get_me_with_invalid_token(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    // Create user
    let user = UserFixture::new()
        .email("user@test.com")
        .insert(&pool)
        .await
        .expect("Failed to create user");
    
    let invalid_token = AuthFixture::generate_invalid_token(user.id);
    
    let response = make_request(app, "GET", "/api/auth/me", Some(&invalid_token), None).await;
    
    // Should fail with unauthorized
    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_get_me_without_token(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let response = make_request(app, "GET", "/api/auth/me", None, None).await;
    
    // Should fail with unauthorized
    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_get_me_with_malformed_token(pool: PgPool) {
    let app = build_app(pool.clone()).await;
    
    let malformed_token = "not.a.valid.jwt.token";
    
    let response = make_request(app, "GET", "/api/auth/me", Some(malformed_token), None).await;
    
    // Should fail with unauthorized
    assert_status(&response, StatusCode::UNAUTHORIZED);
}
