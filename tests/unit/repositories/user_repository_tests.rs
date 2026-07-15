use socs_backend::repositories::user_repository;
use socs_backend::models::user::UserRole;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_user(pool: PgPool) {
    let name = "Test User";
    let email = format!("test_{}@example.com", Uuid::new_v4());
    let password_hash = "hashed_password";

    let result = user_repository::create(&pool, name, &email, password_hash).await;
    assert!(result.is_ok());

    let user = result.unwrap();
    assert_eq!(user.name, name);
    assert_eq!(user.email, email);
    // Newly created users default to MEMBER role
    assert_eq!(user.roles, vec![UserRole::Member]);
    assert!(user.email_verified_at.is_none());
}

#[sqlx::test]
async fn test_find_by_email(pool: PgPool) {
    let email = format!("find_{}@example.com", Uuid::new_v4());
    user_repository::create(&pool, "Find Me", &email, "hash").await.unwrap();

    let result = user_repository::find_by_email(&pool, &email).await;
    assert!(result.is_ok());
    
    let user = result.unwrap();
    assert!(user.is_some());
    assert_eq!(user.unwrap().email, email);
}

#[sqlx::test]
async fn test_find_by_email_not_found(pool: PgPool) {
    let email = "nonexistent@example.com";
    
    let result = user_repository::find_by_email(&pool, email).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[sqlx::test]
async fn test_find_by_id(pool: PgPool) {
    let email = format!("find_id_{}@example.com", Uuid::new_v4());
    let created = user_repository::create(&pool, "Find By ID", &email, "hash").await.unwrap();

    let result = user_repository::find_by_id(&pool, created.id).await;
    assert!(result.is_ok());
    
    let user = result.unwrap();
    assert!(user.is_some());
    assert_eq!(user.unwrap().id, created.id);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("delete_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Delete Me", &email, "hash").await.unwrap();

    // 1. Soft delete
    let deleted = user_repository::soft_delete(&pool, user.id).await.unwrap();
    assert!(deleted);

    // 2. Regular find should not return deleted user
    let found = user_repository::find_by_id(&pool, user.id).await.unwrap();
    assert!(found.is_none());

    // 3. Find with deleted should return the user
    let found_deleted = user_repository::find_by_id_with_deleted(&pool, user.id).await.unwrap();
    assert!(found_deleted.is_some());
    
    // 4. Restore
    let restored = user_repository::restore(&pool, user.id).await.unwrap();
    assert!(restored);
    
    // 5. Regular find should work again
    let found_restored = user_repository::find_by_id(&pool, user.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("perm_delete_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Perm Delete Me", &email, "hash").await.unwrap();

    user_repository::soft_delete(&pool, user.id).await.unwrap();
    let deleted = user_repository::permanent_delete(&pool, user.id).await.unwrap();
    assert!(deleted);

    // Should not be found even with deleted
    let found = user_repository::find_by_id_with_deleted(&pool, user.id).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test]
async fn test_update_profile_picture(pool: PgPool) {
    let email = format!("pic_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Pic User", &email, "hash").await.unwrap();

    let pic_url = "https://example.com/pic.png";
    let updated = user_repository::update_profile_picture(&pool, user.id, pic_url).await;
    assert!(updated.is_ok());

    let found = user_repository::find_by_id(&pool, user.id).await.unwrap().unwrap();
    assert_eq!(found.profile_picture.unwrap(), pic_url);
    
    let matches = user_repository::profile_picture_matches(&pool, user.id, pic_url).await.unwrap();
    assert!(matches);
    
    let cleared = user_repository::clear_profile_picture(&pool, user.id, pic_url).await.unwrap();
    assert!(cleared);
    
    let final_user = user_repository::find_by_id(&pool, user.id).await.unwrap().unwrap();
    assert!(final_user.profile_picture.is_none());
}

#[sqlx::test]
async fn test_mark_email_verified(pool: PgPool) {
    let email = format!("verify_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Verify Me", &email, "hash").await.unwrap();
    
    assert!(user.email_verified_at.is_none());
    
    let verified = user_repository::mark_email_verified(&pool, user.id).await.unwrap();
    assert!(verified.email_verified_at.is_some());
}

#[sqlx::test]
async fn test_update_password(pool: PgPool) {
    let email = format!("pass_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Pass User", &email, "old_hash").await.unwrap();
    
    let new_hash = "new_super_secret_hash";
    let updated = user_repository::update_password(&pool, user.id, new_hash).await.unwrap();
    
    // We would need to retrieve it to verify, but password is skip_serializing.
    // We can just verify the function returned success, or run a raw query to check.
    assert_eq!(updated.id, user.id);
    
    // Using sqlx to check actual db value
    let db_hash: String = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
    assert_eq!(db_hash, new_hash);
}
