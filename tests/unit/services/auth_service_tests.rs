use socs_backend::{
    dto::auth_dto::{LoginDto, RegisterDto},
    error::ApiError,
    services::auth_service,
};
use sqlx::PgPool;
use uuid::Uuid;

const JWT_SECRET: &str = "test_secret_for_auth_service";
const JWT_EXPIRES_IN: i64 = 3600;

#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let email = format!("test_reg_{}@example.com", Uuid::new_v4());
    
    let dto = RegisterDto {
        name: "Test User".to_string(),
        email: email.clone(),
        password: "password123".to_string(),
    };

    let result = auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, dto).await;
    assert!(result.is_ok());

    let auth_res = result.unwrap();
    assert_eq!(auth_res.user.email, email);
    assert_eq!(auth_res.user.name, "Test User");
    assert!(!auth_res.token.is_empty());
}

#[sqlx::test]
async fn test_register_duplicate_email(pool: PgPool) {
    let email = format!("test_dup_{}@example.com", Uuid::new_v4());
    
    let dto1 = RegisterDto {
        name: "Test User 1".to_string(),
        email: email.clone(),
        password: "password123".to_string(),
    };

    let dto2 = RegisterDto {
        name: "Test User 2".to_string(),
        email: email.clone(),
        password: "password456".to_string(),
    };

    // First one succeeds
    assert!(auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, dto1).await.is_ok());

    // Second one fails with Conflict
    let result = auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, dto2).await;
    assert!(matches!(result, Err(ApiError::Conflict(_))));
}

#[sqlx::test]
async fn test_login_success(pool: PgPool) {
    let email = format!("test_login_{}@example.com", Uuid::new_v4());
    let password = "my_secure_password".to_string();
    
    let reg_dto = RegisterDto {
        name: "Login User".to_string(),
        email: email.clone(),
        password: password.clone(),
    };

    auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, reg_dto).await.unwrap();

    let login_dto = LoginDto {
        email: email.clone(),
        password: password.clone(),
    };

    let result = auth_service::login(&pool, JWT_SECRET, JWT_EXPIRES_IN, login_dto).await;
    assert!(result.is_ok());

    let auth_res = result.unwrap();
    assert_eq!(auth_res.user.email, email);
    assert!(!auth_res.token.is_empty());
}

#[sqlx::test]
async fn test_login_invalid_password(pool: PgPool) {
    let email = format!("test_inv_pass_{}@example.com", Uuid::new_v4());
    let password = "my_secure_password".to_string();
    
    let reg_dto = RegisterDto {
        name: "Login User".to_string(),
        email: email.clone(),
        password: password.clone(),
    };

    auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, reg_dto).await.unwrap();

    let login_dto = LoginDto {
        email: email.clone(),
        password: "wrong_password".to_string(),
    };

    let result = auth_service::login(&pool, JWT_SECRET, JWT_EXPIRES_IN, login_dto).await;
    assert!(matches!(result, Err(ApiError::Unauthorized(_))));
}

#[sqlx::test]
async fn test_login_user_not_found(pool: PgPool) {
    let login_dto = LoginDto {
        email: format!("non_existent_{}@example.com", Uuid::new_v4()),
        password: "password123".to_string(),
    };

    let result = auth_service::login(&pool, JWT_SECRET, JWT_EXPIRES_IN, login_dto).await;
    assert!(matches!(result, Err(ApiError::Unauthorized(_))));
}

#[sqlx::test]
async fn test_get_user_by_id(pool: PgPool) {
    let email = format!("test_get_user_{}@example.com", Uuid::new_v4());
    
    let dto = RegisterDto {
        name: "Get User".to_string(),
        email: email.clone(),
        password: "password123".to_string(),
    };

    let auth_res = auth_service::register(&pool, JWT_SECRET, JWT_EXPIRES_IN, dto).await.unwrap();
    let user_id = auth_res.user.id;

    let result = auth_service::get_user_by_id(&pool, user_id).await;
    assert!(result.is_ok());

    let safe_user = result.unwrap();
    assert_eq!(safe_user.id, user_id);
    assert_eq!(safe_user.email, email);
    assert_eq!(safe_user.name, "Get User");
}

#[sqlx::test]
async fn test_get_user_by_id_not_found(pool: PgPool) {
    let result = auth_service::get_user_by_id(&pool, Uuid::new_v4()).await;
    assert!(matches!(result, Err(ApiError::NotFound(_))));
}
