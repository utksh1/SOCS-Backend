use chrono::{Utc, Duration};
use socs_backend::repositories::{
    user_repository, verification_token_repository, rate_limit_repository,
};
use socs_backend::services::verification_service::{
    generate_secure_token, hash_token, verify_email_token, reset_password_with_token, cleanup_expired_data,
};
use socs_backend::services::email_service::EmailService;
use socs_backend::models::verification_token::TokenType;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

fn setup_email_env() {
    env::set_var("SMTP_HOST", "localhost");
    env::set_var("SMTP_PORT", "2525");
    env::set_var("SMTP_USERNAME", "test@example.com");
    env::set_var("SMTP_PASSWORD", "testpass");
}

#[sqlx::test]
async fn test_verify_email_token(pool: PgPool) {
    let email = format!("verify_test_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Verify User", &email, "hash").await.unwrap();
    
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    
    // Create token
    verification_token_repository::create(
        &pool,
        user.id,
        &token_hash,
        TokenType::EmailVerification,
        Utc::now() + Duration::minutes(30),
    ).await.unwrap();
    
    // Verify it
    let result = verify_email_token(&pool, &token).await;
    assert!(result.is_ok());
    
    let updated_user = result.unwrap();
    assert!(updated_user.email_verified_at.is_some());
    
    // Try to verify again, should fail
    let result2 = verify_email_token(&pool, &token).await;
    assert!(result2.is_err());
}

#[sqlx::test]
async fn test_verify_email_token_invalid_type(pool: PgPool) {
    let email = format!("verify_inv_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Verify User Inv", &email, "hash").await.unwrap();
    
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    
    // Create token of WRONG type
    verification_token_repository::create(
        &pool,
        user.id,
        &token_hash,
        TokenType::PasswordReset,
        Utc::now() + Duration::minutes(30),
    ).await.unwrap();
    
    // Verify it
    let result = verify_email_token(&pool, &token).await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn test_reset_password_with_token(pool: PgPool) {
    setup_email_env();
    let email_service = EmailService::new().unwrap();
    
    let email = format!("reset_test_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Reset User", &email, "old_hash").await.unwrap();
    
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    
    // Create token
    verification_token_repository::create(
        &pool,
        user.id,
        &token_hash,
        TokenType::PasswordReset,
        Utc::now() + Duration::minutes(30),
    ).await.unwrap();
    
    // Reset it
    let new_password = "new_secure_password";
    let result = reset_password_with_token(&pool, &email_service, &token, new_password).await;
    assert!(result.is_ok());
    
    let updated_user = result.unwrap();
    assert_ne!(updated_user.password, "old_hash");
    
    // Try to reset again, should fail
    let result2 = reset_password_with_token(&pool, &email_service, &token, new_password).await;
    assert!(result2.is_err());
}

#[sqlx::test]
async fn test_cleanup_expired_data(pool: PgPool) {
    let email = format!("cleanup_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Cleanup User", &email, "hash").await.unwrap();
    
    // Insert an expired token
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    verification_token_repository::create(
        &pool,
        user.id,
        &token_hash,
        TokenType::EmailVerification,
        Utc::now() - Duration::hours(1), // Expired
    ).await.unwrap();
    
    // Insert old rate limit
    let _ = rate_limit_repository::check_and_increment(
        &pool,
        "test@example.com",
        TokenType::EmailVerification,
        3,
        1,
    ).await.unwrap();
    
    sqlx::query("UPDATE token_rate_limits SET window_start = NOW() - INTERVAL '25 hours'")
        .execute(&pool)
        .await
        .unwrap();

    let result = cleanup_expired_data(&pool).await;
    assert!(result.is_ok());
    
    // Check token is deleted
    let found_token = verification_token_repository::find_by_hash(&pool, &token_hash).await.unwrap();
    assert!(found_token.is_none());
}
