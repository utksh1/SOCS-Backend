use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ApiError;
use crate::AppState;

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

/// Rate limit middleware for authentication endpoints (login, register, password)
/// Limits: 20 requests per 15 minutes per IP
pub async fn auth_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = RateLimitConfig {
        capacity: 20.0,
        refill_rate: 20.0 / (15.0 * 60.0), // 20 tokens per 15 minutes
        key_prefix: "rate_limit:auth:ip".to_string(),
        ttl: 30 * 60, // 30 minutes
    };

    let ip = extract_ip(&request);
    let key = format!("{}", ip);

    let mut redis = state.redis.clone();
    
    // Check rate limit (fail-open on Redis errors)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed for auth endpoint, allowing request: {:?}", e);
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for IP {} on auth endpoint, retry after {} seconds", ip, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}

/// Rate limit middleware for public endpoints
/// Limits: 500 requests per minute per IP
pub async fn public_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = RateLimitConfig {
        capacity: 500.0,
        refill_rate: 500.0 / 60.0, // 500 tokens per minute
        key_prefix: "rate_limit:public:ip".to_string(),
        ttl: 2 * 60, // 2 minutes
    };

    let ip = extract_ip(&request);
    let key = format!("{}", ip);

    let mut redis = state.redis.clone();
    
    // Check rate limit (fail-open on Redis errors)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed for public endpoint, allowing request: {:?}", e);
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for IP {} on public endpoint, retry after {} seconds", ip, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}
