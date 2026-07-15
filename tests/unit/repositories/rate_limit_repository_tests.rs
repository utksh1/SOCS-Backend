use chrono::Duration;
use socs_backend::repositories::rate_limit_repository;
use socs_backend::models::verification_token::TokenType;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_check_and_increment(pool: PgPool) {
    let email = format!("rl_{}@example.com", Uuid::new_v4());
    
    // First attempt should pass
    let allowed = rate_limit_repository::check_and_increment(&pool, &email, TokenType::EmailVerification, 2, 1).await.unwrap();
    assert!(allowed);

    let count = rate_limit_repository::get_attempt_count(&pool, &email, TokenType::EmailVerification, 1).await.unwrap();
    assert_eq!(count, 1);

    // Second attempt should pass
    let allowed = rate_limit_repository::check_and_increment(&pool, &email, TokenType::EmailVerification, 2, 1).await.unwrap();
    assert!(allowed);

    let count = rate_limit_repository::get_attempt_count(&pool, &email, TokenType::EmailVerification, 1).await.unwrap();
    assert_eq!(count, 2);

    // Third attempt should fail
    let allowed = rate_limit_repository::check_and_increment(&pool, &email, TokenType::EmailVerification, 2, 1).await.unwrap();
    assert!(!allowed);

    let count = rate_limit_repository::get_attempt_count(&pool, &email, TokenType::EmailVerification, 1).await.unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test]
async fn test_cleanup_old_records(pool: PgPool) {
    let email = format!("rl_clean_{}@example.com", Uuid::new_v4());
    
    // Create a record that will be considered "old" if we clean up with older_than_hours = 0
    // Actually we can't easily force an old record via the public API because it uses Utc::now()
    // but we can manually insert one for testing
    let window_start = chrono::Utc::now() - Duration::hours(5);
    
    sqlx::query(
        "INSERT INTO token_rate_limits (email, token_type, attempt_count, window_start) VALUES ($1, $2, 1, $3)"
    )
    .bind(&email)
    .bind(TokenType::EmailVerification.as_str())
    .bind(window_start)
    .execute(&pool)
    .await
    .unwrap();

    let cleaned = rate_limit_repository::cleanup_old_records(&pool, 1).await.unwrap();
    assert!(cleaned >= 1);
}
