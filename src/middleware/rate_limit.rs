use axum::{
    extract::{connect_info::ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use redis::Script;
use std::{
    net::{IpAddr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

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

/// Check rate limit and consume a token if available
pub async fn check_rate_limit(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    config: &RateLimitConfig,
) -> Result<(bool, u64), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let tokens_key = format!("{}:{}:tokens", config.key_prefix, key);
    let last_refill_key = format!("{}:{}:last_refill", config.key_prefix, key);

    // The read/refill/write sequence must be one Redis operation. Separate GET
    // and SET calls let concurrent requests spend the same token repeatedly.
    let script = Script::new(
        r#"
        local tokens = tonumber(redis.call('GET', KEYS[1]))
        local last_refill = tonumber(redis.call('GET', KEYS[2]))
        local now = tonumber(ARGV[1])
        local capacity = tonumber(ARGV[2])
        local refill_rate = tonumber(ARGV[3])
        local ttl = tonumber(ARGV[4])

        if tokens == nil or last_refill == nil then
            tokens = capacity
        else
            local elapsed = math.max(0, now - last_refill)
            tokens = math.min(capacity, tokens + elapsed * refill_rate)
        end

        local allowed = 0
        local retry_after = 0
        if tokens >= 1 then
            tokens = tokens - 1
            allowed = 1
        else
            retry_after = math.ceil((1 - tokens) / refill_rate)
        end

        redis.call('SETEX', KEYS[1], ttl, tokens)
        redis.call('SETEX', KEYS[2], ttl, now)
        return { allowed, retry_after }
        "#,
    );

    let (allowed, retry_after): (i64, i64) = script
        .key(tokens_key)
        .key(last_refill_key)
        .arg(now)
        .arg(config.capacity)
        .arg(config.refill_rate)
        .arg(config.ttl)
        .invoke_async(redis)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Redis rate-limit script failed");
            ApiError::ServiceUnavailable("Rate limiting is temporarily unavailable".to_string())
        })?;

    Ok((allowed == 1, retry_after.max(0) as u64))
}

/// Extract the client address. Forwarded headers are accepted only when the
/// deployment explicitly trusts its reverse proxy; otherwise clients could
/// forge a new rate-limit bucket with each request.
pub fn extract_ip<B>(request: &axum::http::Request<B>) -> String {
    let client_ip = if let Some(ConnectInfo(address)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        address.ip()
    } else {
        return "unknown".to_string();
    };

    let trust_proxy_headers = std::env::var("TRUST_PROXY_HEADERS")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if trust_proxy_headers {
        let trusted_proxies_env = std::env::var("TRUSTED_PROXIES").unwrap_or_default();
        let is_trusted_proxy = if trusted_proxies_env.is_empty() {
            true // Backward compatibility: if TRUST_PROXY_HEADERS=true but no list, trust all
        } else {
            trusted_proxies_env
                .split(',')
                .map(str::trim)
                .any(|ip_str| ip_str.parse::<IpAddr>().map_or(false, |ip| ip == client_ip))
        };

        if is_trusted_proxy {
            if let Some(forwarded) = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|header| header.to_str().ok())
            {
                // Proxies append the immediately preceding address. Taking the
                // rightmost valid address prevents a client-supplied leading value
                // from overriding the proxy-observed address.
                if let Some(ip) = forwarded
                    .split(',')
                    .rev()
                    .map(str::trim)
                    .find_map(|value| value.parse::<IpAddr>().ok())
                {
                    return ip.to_string();
                }
            }

            if let Some(ip) = request
                .headers()
                .get("x-real-ip")
                .and_then(|header| header.to_str().ok())
                .and_then(|value| value.trim().parse::<IpAddr>().ok())
            {
                return ip.to_string();
            }
        }
    }

    client_ip.to_string()
}

/// Rate limit middleware for authentication endpoints (login, register, password)
/// Limits: 20 requests per 15 minutes per IP
pub async fn auth_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if !state.config.rate_limit_enabled {
        return Ok(next.run(request).await);
    }
    let config = RateLimitConfig {
        capacity: 20.0,
        refill_rate: 20.0 / (15.0 * 60.0), // 20 tokens per 15 minutes
        key_prefix: "rate_limit:auth:ip".to_string(),
        ttl: 30 * 60, // 30 minutes
    };

    let ip = extract_ip(&request);
    let key = ip.clone();

    let mut redis = state.redis.clone().ok_or_else(|| {
        ApiError::ServiceUnavailable("Rate limiting is temporarily unavailable".to_string())
    })?;
    
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = ?e, "Rate limit check failed for auth endpoint");
            return Err(e);
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
    if !state.config.rate_limit_enabled {
        return Ok(next.run(request).await);
    }
    let config = RateLimitConfig {
        capacity: 500.0,
        refill_rate: 500.0 / 60.0, // 500 tokens per minute
        key_prefix: "rate_limit:public:ip".to_string(),
        ttl: 2 * 60, // 2 minutes
    };

    let ip = extract_ip(&request);
    let key = ip.clone();

    let Some(mut redis) = state.redis.clone() else {
        tracing::error!("Rate limiting is enabled but Redis is unavailable for a public endpoint");
        let fail_closed = std::env::var("RATE_LIMIT_FAIL_CLOSED").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(true);
        if fail_closed {
            return Err(ApiError::ServiceUnavailable("Rate limiting is temporarily unavailable".to_string()));
        }
        return Ok(next.run(request).await);
    };
    
    // Check rate limit (fail-closed configurable)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            let fail_closed = std::env::var("RATE_LIMIT_FAIL_CLOSED").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(true);
            if fail_closed {
                tracing::error!(error = ?e, "Rate limit check failed for public endpoint, blocking request");
                return Err(e);
            }
            tracing::error!(error = ?e, "Rate limit check failed for public endpoint, allowing request");
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for IP {} on public endpoint, retry after {} seconds", ip, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}

/// Rate limit middleware for authenticated endpoints
/// Limits: 5000 requests per minute per user
pub async fn authenticated_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if !state.config.rate_limit_enabled {
        return Ok(next.run(request).await);
    }
    let config = RateLimitConfig {
        capacity: 5000.0,
        refill_rate: 5000.0 / 60.0, // 5000 tokens per minute
        key_prefix: "rate_limit:user".to_string(),
        ttl: 2 * 60, // 2 minutes
    };

    // Extract user from request extensions (set by auth middleware)
    let user = request.extensions().get::<crate::models::user::SafeUser>()
        .ok_or_else(|| ApiError::Unauthorized("User not authenticated".to_string()))?;

    let key = format!("{}", user.id);

    let Some(mut redis) = state.redis.clone() else {
        tracing::error!("Rate limiting is enabled but Redis is unavailable for an authenticated endpoint");
        let fail_closed = std::env::var("RATE_LIMIT_FAIL_CLOSED").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(true);
        if fail_closed {
            return Err(ApiError::ServiceUnavailable("Rate limiting is temporarily unavailable".to_string()));
        }
        return Ok(next.run(request).await);
    };
    
    // Check rate limit (fail-closed configurable)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            let fail_closed = std::env::var("RATE_LIMIT_FAIL_CLOSED").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(true);
            if fail_closed {
                tracing::error!(error = ?e, "Rate limit check failed for authenticated endpoint, blocking request");
                return Err(e);
            }
            tracing::error!(error = ?e, "Rate limit check failed for authenticated endpoint, allowing request");
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for user {} on authenticated endpoint, retry after {} seconds", user.id, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}

/// Rate limit storage-consuming upload endpoints independently of the normal
/// authenticated API budget. Ten 5 MiB uploads per hour keeps a compromised
/// account from being used as an object-storage relay.
pub async fn upload_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if !state.config.rate_limit_enabled {
        return Ok(next.run(request).await);
    }

    let user = request
        .extensions()
        .get::<crate::models::user::SafeUser>()
        .ok_or_else(|| ApiError::Unauthorized("User not authenticated".to_string()))?;
    let config = RateLimitConfig {
        capacity: 10.0,
        refill_rate: 10.0 / (60.0 * 60.0),
        key_prefix: "rate_limit:upload:user".to_string(),
        ttl: 2 * 60 * 60,
    };
    let mut redis = state.redis.clone().ok_or_else(|| {
        ApiError::ServiceUnavailable("Upload rate limiting is temporarily unavailable".to_string())
    })?;

    let (allowed, retry_after) = check_rate_limit(&mut redis, &user.id.to_string(), &config).await?;
    if !allowed {
        tracing::warn!(user_id = %user.id, retry_after, "Upload rate limit exceeded");
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::{extract::connect_info::ConnectInfo, http::Request};

    use super::extract_ip;

    #[test]
    fn connection_address_cannot_be_overridden_by_forwarded_headers() {
        let mut request = Request::builder()
            .header("x-forwarded-for", "203.0.113.1")
            .body(())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            4321,
        )));

        assert_eq!(extract_ip(&request), "127.0.0.1");
    }
}
