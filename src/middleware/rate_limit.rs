use axum::extract::Request;
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ApiError;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of tokens (capacity)
    pub capacity: f64,
    /// Tokens added per second
    pub refill_rate: f64,
    /// Redis key prefix
    pub key_prefix: String,
    /// TTL for Redis keys in seconds
    pub ttl: u64,
}

/// Token bucket state stored in Redis
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: i64,
}

/// Check rate limit and consume a token if available
pub async fn check_rate_limit(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    config: &RateLimitConfig,
) -> Result<(bool, u64), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time should be after UNIX_EPOCH")
        .as_secs() as i64;

    // Get current bucket state from Redis
    let tokens_key = format!("{}:{}:tokens", config.key_prefix, key);
    let last_refill_key = format!("{}:{}:last_refill", config.key_prefix, key);

    let tokens: Option<f64> = redis.get(&tokens_key).await
        .map_err(|e| {
            tracing::error!("Failed to get tokens from Redis: {}", e);
            ApiError::InternalServerError
        })?;
    let last_refill: Option<i64> = redis.get(&last_refill_key).await
        .map_err(|e| {
            tracing::error!("Failed to get last_refill from Redis: {}", e);
            ApiError::InternalServerError
        })?;

    // Initialize or calculate new token count
    let (mut current_tokens, last_refill_time) = match (tokens, last_refill) {
        (Some(t), Some(lr)) => {
            // Calculate elapsed time and refill tokens
            let elapsed = (now - lr) as f64;
            let new_tokens = (t + (elapsed * config.refill_rate)).min(config.capacity);
            (new_tokens, now)
        }
        _ => {
            // First request - initialize with full capacity
            (config.capacity, now)
        }
    };

    // Check if we have tokens available
    if current_tokens >= 1.0 {
        // Consume one token
        current_tokens -= 1.0;

        // Update Redis with explicit type annotations
        redis.set_ex::<_, _, ()>(&tokens_key, current_tokens, config.ttl).await
            .map_err(|e| {
                tracing::error!("Failed to update Redis tokens: {}", e);
                ApiError::InternalServerError
            })?;
        redis.set_ex::<_, _, ()>(&last_refill_key, last_refill_time, config.ttl).await
            .map_err(|e| {
                tracing::error!("Failed to update Redis last_refill: {}", e);
                ApiError::InternalServerError
            })?;

        Ok((true, 0)) // Allowed
    } else {
        // Calculate retry-after time (seconds until 1 token available)
        let tokens_needed = 1.0 - current_tokens;
        let retry_after = (tokens_needed / config.refill_rate).ceil() as u64;

        Ok((false, retry_after)) // Denied
    }
}

/// Extract client IP address from request headers
pub fn extract_ip(request: &Request) -> String {
    // Check X-Forwarded-For header
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback
    "unknown".to_string()
}
