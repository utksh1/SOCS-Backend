use socs_backend::services::email_service::EmailService;
use std::env;

#[tokio::test]
async fn test_email_service_new_success() {
    env::set_var("SMTP_HOST", "localhost");
    env::set_var("SMTP_PORT", "2525");
    env::set_var("SMTP_USERNAME", "test@example.com");
    env::set_var("SMTP_PASSWORD", "testpass");
    env::set_var("FROM_EMAIL", "noreply@example.com");
    
    let result = EmailService::new();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_email_service_new_missing_username() {
    env::set_var("SMTP_HOST", "localhost");
    env::set_var("SMTP_PORT", "2525");
    env::remove_var("SMTP_USERNAME");
    env::set_var("SMTP_PASSWORD", "testpass");
    
    let result = EmailService::new();
    assert!(result.is_err());
}
