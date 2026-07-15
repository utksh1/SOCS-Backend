use socs_backend::services::r2_service::R2Client;
use std::env;

#[test]
fn test_r2_service_new_success() {
    env::set_var("R2_ACCOUNT_ID", "test_account");
    env::set_var("R2_ACCESS_KEY_ID", "test_access");
    env::set_var("R2_SECRET_ACCESS_KEY", "test_secret");
    env::set_var("R2_BUCKET_NAME", "socs-images");
    env::set_var("R2_PUBLIC_URL", "https://images.example.com");
    
    let result = R2Client::new();
    assert!(result.is_ok());
}

#[test]
fn test_r2_service_new_missing_account_id() {
    env::remove_var("R2_ACCOUNT_ID");
    env::set_var("R2_ACCESS_KEY_ID", "test_access");
    env::set_var("R2_SECRET_ACCESS_KEY", "test_secret");
    env::set_var("R2_PUBLIC_URL", "https://images.example.com");
    
    let result = R2Client::new();
    assert!(result.is_err());
}
