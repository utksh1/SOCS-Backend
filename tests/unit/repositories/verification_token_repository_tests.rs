use chrono::{Utc, Duration};
use socs_backend::repositories::{verification_token_repository, user_repository};
use socs_backend::models::verification_token::TokenType;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_and_find_by_hash(pool: PgPool) {
    let email = format!("vt_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "VT User", &email, "hash").await.unwrap();

    let hash = format!("hash_{}", Uuid::new_v4());
    let expires = Utc::now() + Duration::hours(1);

    let token = verification_token_repository::create(
        &pool,
        user.id,
        &hash,
        TokenType::EmailVerification,
        expires,
    ).await.unwrap();

    assert_eq!(token.user_id, user.id);
    assert_eq!(token.token_hash, hash);
    assert_eq!(token.token_type, TokenType::EmailVerification);
    assert!(token.used_at.is_none());

    let found = verification_token_repository::find_by_hash(&pool, &hash).await.unwrap().unwrap();
    assert_eq!(found.id, token.id);
}

#[sqlx::test]
async fn test_mark_as_used(pool: PgPool) {
    let email = format!("vt_used_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "VT User Used", &email, "hash").await.unwrap();

    let hash = format!("hash_{}", Uuid::new_v4());
    let token = verification_token_repository::create(
        &pool, user.id, &hash, TokenType::PasswordReset, Utc::now() + Duration::hours(1)
    ).await.unwrap();

    let used_token = verification_token_repository::mark_as_used(&pool, token.id).await.unwrap().unwrap();
    assert!(used_token.used_at.is_some());

    // Second time should return None
    let used_again = verification_token_repository::mark_as_used(&pool, token.id).await.unwrap();
    assert!(used_again.is_none());
}

#[sqlx::test]
async fn test_delete_user_tokens(pool: PgPool) {
    let email = format!("vt_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "VT User Del", &email, "hash").await.unwrap();

    verification_token_repository::create(&pool, user.id, &format!("h1_{}", Uuid::new_v4()), TokenType::EmailVerification, Utc::now() + Duration::hours(1)).await.unwrap();
    verification_token_repository::create(&pool, user.id, &format!("h2_{}", Uuid::new_v4()), TokenType::EmailVerification, Utc::now() + Duration::hours(1)).await.unwrap();
    verification_token_repository::create(&pool, user.id, &format!("h3_{}", Uuid::new_v4()), TokenType::PasswordReset, Utc::now() + Duration::hours(1)).await.unwrap();

    let count = verification_token_repository::count_active_tokens(&pool, user.id, TokenType::EmailVerification).await.unwrap();
    assert_eq!(count, 2);

    let deleted = verification_token_repository::delete_user_tokens(&pool, user.id, TokenType::EmailVerification).await.unwrap();
    assert_eq!(deleted, 2);

    let count_after = verification_token_repository::count_active_tokens(&pool, user.id, TokenType::EmailVerification).await.unwrap();
    assert_eq!(count_after, 0);

    let pr_count = verification_token_repository::count_active_tokens(&pool, user.id, TokenType::PasswordReset).await.unwrap();
    assert_eq!(pr_count, 1);
}

#[sqlx::test]
async fn test_cleanup_expired(pool: PgPool) {
    let email = format!("vt_clean_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "VT User Clean", &email, "hash").await.unwrap();

    // Expired
    verification_token_repository::create(&pool, user.id, &format!("h1_{}", Uuid::new_v4()), TokenType::EmailVerification, Utc::now() - Duration::hours(1)).await.unwrap();
    // Valid
    let t2 = verification_token_repository::create(&pool, user.id, &format!("h2_{}", Uuid::new_v4()), TokenType::EmailVerification, Utc::now() + Duration::hours(1)).await.unwrap();
    // Used
    let t3 = verification_token_repository::create(&pool, user.id, &format!("h3_{}", Uuid::new_v4()), TokenType::EmailVerification, Utc::now() + Duration::hours(1)).await.unwrap();
    verification_token_repository::mark_as_used(&pool, t3.id).await.unwrap();

    let cleaned = verification_token_repository::cleanup_expired(&pool).await.unwrap();
    assert!(cleaned >= 2); // At least the two we just made, possibly others if parallel runs

    let found_valid = verification_token_repository::find_by_hash(&pool, &t2.token_hash).await.unwrap();
    assert!(found_valid.is_some());
}
