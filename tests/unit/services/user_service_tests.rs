use chrono::{Utc, Duration};
use socs_backend::repositories::{
    user_repository,
    verification_token_repository,
};
use socs_backend::services::user_service;
use socs_backend::models::verification_token::TokenType;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_soft_delete_user_cascades(pool: PgPool) {
    let email = format!("us_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "US User", &email, "hash").await.unwrap();

    // Create a verification token for this user
    let hash = format!("token_{}", Uuid::new_v4());
    let token = verification_token_repository::create(
        &pool,
        user.id,
        &hash,
        TokenType::EmailVerification,
        Utc::now() + Duration::hours(1),
    ).await.unwrap();

    // Create a project and add this user as a contributor
    // Add team contribution
    sqlx::query(
        "INSERT INTO team_contributions (team_member_id, contribution_type, title, description, url, contribution_date) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user.id)
    .bind("code")
    .bind("My Contribution")
    .bind("Desc")
    .bind("http://example.com")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .unwrap();

    // Now soft delete the user
    let result = user_service::soft_delete_user(&pool, user.id).await;
    assert!(result.is_ok());

    // User should be soft deleted
    let found_user = user_repository::find_by_id(&pool, user.id).await.unwrap();
    assert!(found_user.is_none()); // find_by_id filters by deleted_at IS NULL

    // Token should be hard deleted
    let found_token = verification_token_repository::find_by_hash(&pool, &token.token_hash).await.unwrap();
    assert!(found_token.is_none());

    // Team contribution should be soft deleted
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM team_contributions WHERE team_member_id = $1 AND deleted_at IS NULL"
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert_eq!(count.0, 0);
}

#[sqlx::test]
async fn test_restore_user_cascades(pool: PgPool) {
    let email = format!("us_rest_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "US User Rest", &email, "hash").await.unwrap();

    sqlx::query(
        "INSERT INTO team_contributions (team_member_id, contribution_type, title, description, url, contribution_date) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user.id)
    .bind("code")
    .bind("My Contribution")
    .bind("Desc")
    .bind("http://example.com")
    .bind(Utc::now())
    .execute(&pool)
    .await
    .unwrap();

    user_service::soft_delete_user(&pool, user.id).await.unwrap();

    // Now restore
    let result = user_service::restore_user(&pool, user.id).await;
    assert!(result.is_ok());

    let found_user = user_repository::find_by_id(&pool, user.id).await.unwrap();
    assert!(found_user.is_some());

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM team_contributions WHERE team_member_id = $1 AND deleted_at IS NULL"
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert_eq!(count.0, 1);
}

#[sqlx::test]
async fn test_permanent_delete_user(pool: PgPool) {
    let email = format!("us_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "US User Perm", &email, "hash").await.unwrap();

    // Must be soft deleted first
    let result = user_service::permanent_delete_user(&pool, user.id).await;
    assert!(result.is_err()); // should fail because it's not soft deleted

    user_service::soft_delete_user(&pool, user.id).await.unwrap();

    // Now it should succeed
    let result = user_service::permanent_delete_user(&pool, user.id).await;
    assert!(result.is_ok());

    // Should be fully gone from DB
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    
    assert_eq!(count.0, 0);
}
