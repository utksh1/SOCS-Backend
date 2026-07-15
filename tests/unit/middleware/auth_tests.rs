use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::ServiceExt;

use socs_backend::{
    config::env::Config,
    middleware::auth::{auth_middleware, optional_auth_middleware, require_toplead, can_manage_user, can_assign_roles},
    models::user::{SafeUser, UserRole},
    repositories::user_repository,
    utils::jwt,
    AppState,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn test_handler(
    axum::Extension(user): axum::Extension<SafeUser>,
) -> String {
    format!("User ID: {}", user.id)
}

async fn optional_test_handler(
    axum::Extension(user): axum::Extension<Option<SafeUser>>,
) -> String {
    match user {
        Some(u) => format!("User ID: {}", u.id),
        None => "No user".to_string(),
    }
}

fn create_app_state(pool: PgPool, jwt_secret: String) -> AppState {
    let config = Config {
        jwt_secret,
        database_url: "".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        redis_url: "".to_string(),
        rate_limit_enabled: false,
        frontend_url: "".to_string(),
        jwt_expires_in: 3600,
        cors_origin: "".to_string(),
    };
    AppState {
        db: pool,
        config: config,
        redis: None,
    }
}

#[sqlx::test]
async fn test_auth_middleware_valid_token(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());
    
    // Create user
    let email = format!("auth_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Auth User", &email, "hash").await.unwrap();
    let token = jwt::create_token(user.id, UserRole::Member, 0, &secret, 3600).unwrap();

    let app = Router::new()
        .route("/", get(test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_auth_middleware_invalid_token(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());

    let app = Router::new()
        .route("/", get(test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .header("authorization", "Bearer invalid.token.here")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_auth_middleware_missing_token(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());

    let app = Router::new()
        .route("/", get(test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_optional_auth_middleware_with_valid_token(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());
    
    // Create user
    let email = format!("opt_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Opt User", &email, "hash").await.unwrap();
    let token = jwt::create_token(user.id, UserRole::Member, 0, &secret, 3600).unwrap();

    let app = Router::new()
        .route("/", get(optional_test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), optional_auth_middleware))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_optional_auth_middleware_missing_token(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());

    let app = Router::new()
        .route("/", get(optional_test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), optional_auth_middleware))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_require_toplead_with_toplead(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());
    
    // Create user
    let email = format!("lead_{}@example.com", Uuid::new_v4());
    let mut user = user_repository::create(&pool, "Lead User", &email, "hash").await.unwrap();
    
    sqlx::query("UPDATE users SET roles = ARRAY['MEMBER', 'TOPLEAD']::user_role[] WHERE id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();
    
    let token = jwt::create_token(user.id, UserRole::TopLead, 0, &secret, 3600).unwrap();

    let app = Router::new()
        .route("/", get(test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_toplead))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_require_toplead_with_member(pool: PgPool) {
    let secret = "test_secret_12345".to_string();
    let state = create_app_state(pool.clone(), secret.clone());
    
    // Create user
    let email = format!("member_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Member User", &email, "hash").await.unwrap();
    let token = jwt::create_token(user.id, UserRole::Member, 0, &secret, 3600).unwrap();

    let app = Router::new()
        .route("/", get(test_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_toplead))
        .with_state(state);

    let req = Request::builder()
        .uri("/")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_can_manage_user() {
    let mut actor = SafeUser {
        token_version: 0,
        id: Uuid::new_v4(),
        name: "Actor".to_string(),
        email: "actor@example.com".to_string(),
        roles: vec![UserRole::Mentor],
        role: Some(UserRole::Mentor),
        email_verified_at: Some(chrono::Utc::now()),
        slug: None,
        position: None,
        bio: None,
        skills: None,
        github: None,
        linkedin: None,
        avatar_url: None,
        profile_picture: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let mut target = SafeUser {
        token_version: 0,
        id: Uuid::new_v4(),
        name: "Target".to_string(),
        email: "target@example.com".to_string(),
        roles: vec![UserRole::Member],
        role: Some(UserRole::Member),
        email_verified_at: Some(chrono::Utc::now()),
        slug: None,
        position: None,
        bio: None,
        skills: None,
        github: None,
        linkedin: None,
        avatar_url: None,
        profile_picture: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Mentor can manage Member
    assert!(can_manage_user(&actor, &target).is_ok());

    // Member cannot manage Mentor
    assert!(can_manage_user(&target, &actor).is_err());

    // Self can manage self (except roles, checked in can_assign_roles)
    assert!(can_manage_user(&target, &target).is_ok());

    // Mentor cannot manage another Mentor
    let mentor_target = SafeUser {
        id: Uuid::new_v4(),
        roles: vec![UserRole::Mentor],
        role: Some(UserRole::Mentor),
        ..target.clone()
    };
    assert!(can_manage_user(&actor, &mentor_target).is_err());
}

#[test]
fn test_can_assign_roles() {
    let mut actor = SafeUser {
        token_version: 0,
        id: Uuid::new_v4(),
        name: "Actor".to_string(),
        email: "actor@example.com".to_string(),
        roles: vec![UserRole::Mentor],
        role: Some(UserRole::Mentor),
        email_verified_at: Some(chrono::Utc::now()),
        slug: None,
        position: None,
        bio: None,
        skills: None,
        github: None,
        linkedin: None,
        avatar_url: None,
        profile_picture: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Mentor can assign Member
    assert!(can_assign_roles(&actor, &[UserRole::Member]).is_ok());

    // Mentor cannot assign Mentor
    assert!(can_assign_roles(&actor, &[UserRole::Mentor]).is_err());

    // Mentor cannot assign TopLead
    assert!(can_assign_roles(&actor, &[UserRole::TopLead]).is_err());
}
