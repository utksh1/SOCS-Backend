use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "http://127.0.0.1:5001/api";

#[tokio::test]
#[ignore] // Only run when server is running
async fn test_auth_rate_limit_enforcement() {
    let client = reqwest::Client::new();
    
    // Make 20 requests (should all succeed)
    for i in 1..=20 {
        let response = client
            .post(format!("{}/auth/login", BASE_URL))
            .json(&json!({
                "email": "test@example.com",
                "password": "wrongpassword"
            }))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_ne!(response.status(), 429, "Request {} should not be rate limited", i);
    }
    
    // 21st request should be rate limited
    let response = client
        .post(format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "email": "test@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 429, "21st request should be rate limited");
    
    // Check for Retry-After header
    assert!(response.headers().contains_key("retry-after"), "Should include Retry-After header");
}

#[tokio::test]
#[ignore] // Only run when server is running
async fn test_rate_limit_headers() {
    let client = reqwest::Client::new();
    
    let response = client
        .post(format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "email": "test@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await
        .expect("Failed to parse request");
    
    // Check response structure for rate limit error
    if response.status() == 429 {
        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("Rate limit exceeded"));
        assert!(body["retry_after"].is_number());
    }
}

#[tokio::test]
#[ignore] // Only run when server is running
async fn test_token_refill() {
    let client = reqwest::Client::new();
    
    // Consume some tokens
    for _ in 1..=5 {
        client
            .post(format!("{}/auth/login", BASE_URL))
            .json(&json!({
                "email": "refill-test@example.com",
                "password": "wrongpassword"
            }))
            .send()
            .await
            .expect("Failed to send request");
    }
    
    // Wait for tokens to refill (20 tokens per 15 min = 1 token per 45 seconds)
    sleep(Duration::from_secs(50)).await;
    
    // Should be able to make another request
    let response = client
        .post(format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "email": "refill-test@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_ne!(response.status(), 429, "Request should succeed after token refill");
}
