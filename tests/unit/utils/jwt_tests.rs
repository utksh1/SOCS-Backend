use socs_backend::utils::jwt;
use socs_backend::models::user::UserRole;
use uuid::Uuid;

#[test]
fn test_create_token_generates_valid_jwt() {
    let user_id = Uuid::new_v4();
    let role = UserRole::Member;
    let secret = "test_secret_key_12345678";
    let expires_in = 3600;

    let result = jwt::create_token(user_id, role, 1, secret, expires_in);

    assert!(result.is_ok());
    let token = result.unwrap();

    // JWT should have 3 parts separated by dots
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
}

#[test]
fn test_verify_token_validates_signature() {
    let user_id = Uuid::new_v4();
    let role = UserRole::TopLead;
    let secret = "test_secret_key_12345678";
    let expires_in = 3600;

    let token = jwt::create_token(user_id, role.clone(), 1, secret, expires_in).unwrap();
    let result = jwt::verify_token(&token, secret);

    assert!(result.is_ok());
    let claims = result.unwrap();
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.role, role);
}

#[test]
fn test_verify_token_rejects_expired_token() {
    let user_id = Uuid::new_v4();
    let role = UserRole::Member;
    let secret = "test_secret_key_12345678";
    let expires_in = -120; // Expired 120 seconds ago (well past default 60s leeway)

    let token = jwt::create_token(user_id, role, 1, secret, expires_in).unwrap();
    let result = jwt::verify_token(&token, secret);

    assert!(result.is_err());
}

#[test]
fn test_verify_token_rejects_invalid_signature() {
    let user_id = Uuid::new_v4();
    let role = UserRole::Member;
    let secret = "test_secret_key_12345678";
    let wrong_secret = "wrong_secret_key_87654321";
    let expires_in = 3600;

    let token = jwt::create_token(user_id, role, 1, secret, expires_in).unwrap();
    let result = jwt::verify_token(&token, wrong_secret);

    assert!(result.is_err());
}

#[test]
fn test_verify_token_rejects_malformed_token() {
    let secret = "test_secret_key_12345678";
    let malformed_token = "not.a.valid.jwt.token";

    let result = jwt::verify_token(malformed_token, secret);

    assert!(result.is_err());
}

#[test]
fn test_create_token_with_different_roles() {
    let user_id = Uuid::new_v4();
    let secret = "test_secret_key_12345678";
    let expires_in = 3600;

    // Test TopLead role
    let toplead_token = jwt::create_token(user_id, UserRole::TopLead, 1, secret, expires_in).unwrap();
    let toplead_claims = jwt::verify_token(&toplead_token, secret).unwrap();
    assert_eq!(toplead_claims.role, UserRole::TopLead);

    // Test Mentor role
    let mentor_token = jwt::create_token(user_id, UserRole::Mentor, 1, secret, expires_in).unwrap();
    let mentor_claims = jwt::verify_token(&mentor_token, secret).unwrap();
    assert_eq!(mentor_claims.role, UserRole::Mentor);

    // Test Core role
    let core_token = jwt::create_token(user_id, UserRole::Core, 1, secret, expires_in).unwrap();
    let core_claims = jwt::verify_token(&core_token, secret).unwrap();
    assert_eq!(core_claims.role, UserRole::Core);

    // Test Lead role
    let lead_token = jwt::create_token(user_id, UserRole::Lead, 1, secret, expires_in).unwrap();
    let lead_claims = jwt::verify_token(&lead_token, secret).unwrap();
    assert_eq!(lead_claims.role, UserRole::Lead);

    // Test Member role
    let member_token = jwt::create_token(user_id, UserRole::Member, 1, secret, expires_in).unwrap();
    let member_claims = jwt::verify_token(&member_token, secret).unwrap();
    assert_eq!(member_claims.role, UserRole::Member);
}

#[test]
fn test_verify_token_extracts_correct_claims() {
    let user_id = Uuid::new_v4();
    let role = UserRole::Mentor;
    let secret = "test_secret_key_12345678";
    let expires_in = 3600;

    let token = jwt::create_token(user_id, role.clone(), 1, secret, expires_in).unwrap();
    let claims = jwt::verify_token(&token, secret).unwrap();

    // Verify user_id
    assert_eq!(claims.sub, user_id.to_string());

    // Verify role
    assert_eq!(claims.role, role);

    // Verify exp is in the future
    use chrono::Utc;
    let now = Utc::now().timestamp();
    assert!(claims.exp > now);

    // Verify iat is in the past or now
    assert!(claims.iat <= now);
}
