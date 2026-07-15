# Rate Limiting Middleware Design

**Date:** 2026-07-15  
**Status:** Approved  
**Author:** AI Assistant

## Overview

Implement Redis-backed rate limiting middleware using token bucket algorithm to protect the SOCS API against brute-force attacks and DDoS traffic. The system will apply different rate limits based on endpoint type and authentication status.

## Goals

- Protect authentication endpoints (login, register, password change) from brute-force attacks
- Prevent DDoS attacks on all public endpoints
- Provide fair usage limits for authenticated users
- Use Redis for distributed rate limiting across multiple server instances
- Maintain API performance with minimal latency overhead
- Gracefully degrade if Redis becomes unavailable (fail-open)

## Non-Goals

- Web Application Firewall (WAF) features
- IP reputation scoring or blocklists
- Geographic IP filtering
- Bot detection beyond rate limiting

## Architecture

### Core Components

1. **Rate Limiter Module** (`src/middleware/rate_limit.rs`)
   - Token bucket algorithm implementation
   - Redis operations for state management
   - Rate limit configuration structs
   - IP address extraction utilities

2. **Middleware Functions**
   - `auth_rate_limit_middleware` - For login/register/password endpoints (IP-based)
   - `public_rate_limit_middleware` - For public unauthenticated endpoints (IP-based)
   - `authenticated_rate_limit_middleware` - For authenticated endpoints (user-based)

3. **Redis Connection Pool**
   - Shared Redis connection manager in `AppState`
   - Connection string from `REDIS_URL` environment variable
   - Connection pool size: 10 (configurable)

4. **Error Handling**
   - Custom `ApiError::RateLimitExceeded` variant
   - 429 Too Many Requests HTTP response
   - `Retry-After` header with seconds until reset
   - Rate limit info headers (`X-RateLimit-*`)

### Request Flow

```
Incoming Request
    ↓
Rate Limit Middleware
    ↓
Extract Identifier (IP or User ID)
    ↓
Query Redis for Token Bucket State
    ↓
Calculate Token Refill
    ↓
Check Available Tokens
    ↓
  ├─ Tokens >= 1.0
  │    ↓
  │  Consume 1 Token
  │    ↓
  │  Update Redis
  │    ↓
  │  Add Rate Limit Headers
  │    ↓
  │  Pass to Next Middleware
  │
  └─ Tokens < 1.0
       ↓
     Calculate Retry-After
       ↓
     Return 429 Response
```

## Token Bucket Algorithm

### Data Structure

Each rate limit key in Redis stores:
```rust
struct TokenBucket {
    tokens: f64,           // Current available tokens
    last_refill: i64,      // Unix timestamp of last refill
}
```

### Algorithm

1. **Fetch Current State**: Get `tokens` and `last_refill` from Redis
2. **Calculate Elapsed Time**: `elapsed = now - last_refill`
3. **Refill Tokens**: `new_tokens = min(capacity, tokens + (elapsed * refill_rate))`
4. **Check Availability**:
   - If `new_tokens >= 1.0`: Consume 1 token, allow request
   - If `new_tokens < 1.0`: Deny request, calculate retry-after
5. **Update State**: Store new tokens and timestamp in Redis
6. **Set TTL**: Expire keys after 2x window duration

### Rate Limit Configurations

#### Authentication Endpoints (login, register, password change)
- **Capacity**: 20 tokens
- **Refill Rate**: 20 tokens per 15 minutes = 0.0222 tokens/second
- **Window**: 15 minutes
- **Key Pattern**: `rate_limit:auth:ip:{ip_address}`
- **TTL**: 30 minutes
- **Rationale**: Strict limits to prevent brute-force attacks while allowing legitimate retry attempts

#### Public Endpoints (unauthenticated)
- **Capacity**: 500 tokens
- **Refill Rate**: 500 tokens per 60 seconds = 8.33 tokens/second
- **Window**: 1 minute
- **Key Pattern**: `rate_limit:public:ip:{ip_address}`
- **TTL**: 2 minutes
- **Rationale**: Generous limits for browsing, low enough to mitigate DDoS

#### Authenticated Endpoints
- **Capacity**: 5000 tokens
- **Refill Rate**: 5000 tokens per 60 seconds = 83.33 tokens/second
- **Window**: 1 minute
- **Key Pattern**: `rate_limit:user:{user_id}`
- **TTL**: 2 minutes
- **Rationale**: High limits for legitimate authenticated users performing normal operations

## Implementation Details

### Dependencies

Add to `Cargo.toml`:
```toml
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"] }
```

### AppState Changes

```rust
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub redis: redis::aio::ConnectionManager,  // NEW
}
```

### Environment Configuration

Add to `.env`:
```bash
REDIS_URL=redis://localhost:6379
```

Add to `src/config/env.rs`:
```rust
pub struct Config {
    // ... existing fields
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        // ... existing code
        redis_url: env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
    }
}
```

### Middleware Integration

In `src/main.rs`:

```rust
use middleware::rate_limit::{
    auth_rate_limit_middleware,
    public_rate_limit_middleware,
    authenticated_rate_limit_middleware,
};

// Create Redis connection manager
let redis_client = redis::Client::open(config.redis_url.as_str())
    .expect("Failed to create Redis client");
let redis = redis_client
    .get_connection_manager()
    .await
    .expect("Failed to create Redis connection manager");

let state = AppState {
    db,
    config: config.clone(),
    redis,  // NEW
};

// Apply to route groups
let auth_routes = Router::new()
    .route("/register", post(routes::auth::register))
    .route("/login", post(routes::auth::login))
    .route("/change-password", patch(routes::auth::change_password))
    .layer(from_fn_with_state(state.clone(), auth_rate_limit_middleware))
    .layer(toplead_layer.clone());

let public_routes = Router::new()
    .route("/projects", get(routes::projects::list_projects))
    .route("/projects/:id", get(routes::projects::get_project))
    // ... other public endpoints
    .layer(from_fn_with_state(state.clone(), public_rate_limit_middleware));

let protected_routes = Router::new()
    .route("/me", get(routes::auth::get_me))
    .route("/update-name", patch(routes::auth::update_name))
    // ... other protected endpoints
    .layer(from_fn_with_state(state.clone(), authenticated_rate_limit_middleware))
    .layer(auth_layer.clone());
```

### IP Address Extraction

Extract client IP with fallback chain:

1. `X-Forwarded-For` header (first IP in comma-separated list)
2. `X-Real-IP` header
3. Connection peer address

```rust
fn extract_ip(request: &Request) -> String {
    // Check X-Forwarded-For
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }
    
    // Check X-Real-IP
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }
    
    // Fallback to connection peer
    "unknown".to_string()
}
```

## Error Handling

### ApiError Extension

Add to `src/error.rs`:

```rust
pub enum ApiError {
    // ... existing variants
    RateLimitExceeded { retry_after: u64 },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            // ... existing matches
            ApiError::RateLimitExceeded { retry_after } => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "success": false,
                        "error": format!("Rate limit exceeded. Please try again in {} seconds.", retry_after),
                        "retry_after": retry_after,
                    })),
                ).into_response();
                
                response.headers_mut().insert(
                    "Retry-After",
                    retry_after.to_string().parse().unwrap()
                );
                
                response
            }
        }
    }
}
```

### Response Format

**429 Response:**
```json
{
  "success": false,
  "error": "Rate limit exceeded. Please try again in 45 seconds.",
  "retry_after": 45
}
```

**HTTP Headers:**
- `429 Too Many Requests` - Status code
- `Retry-After: 45` - Seconds until tokens refill
- `X-RateLimit-Limit: 500` - Total token capacity
- `X-RateLimit-Remaining: 0` - Tokens currently available
- `X-RateLimit-Reset: 1720987654` - Unix timestamp when full capacity restored

### Graceful Degradation

If Redis operations fail:
- Log error with context (endpoint, IP/user, error message)
- Allow request to proceed (fail-open strategy)
- Track Redis failure metrics
- Prevents Redis outages from taking down entire API

## Testing Strategy

### Unit Tests

- Token bucket refill calculation
- IP address extraction logic
- Rate limit configuration parsing
- Retry-After calculation

### Integration Tests

Test with mock Redis:
- Verify token consumption
- Test rate limit enforcement (should reject after capacity exceeded)
- Verify token refill over time
- Test different limits for auth vs public vs authenticated
- Verify 429 responses with correct headers
- Test graceful degradation when Redis fails

### Manual Testing

```bash
# Test auth endpoint rate limit
for i in {1..25}; do
  curl -X POST http://localhost:5001/api/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"wrong"}' \
    -w "\nStatus: %{http_code}\n"
  sleep 1
done

# Test public endpoint rate limit
for i in {1..600}; do
  curl http://localhost:5001/api/projects -w "\nStatus: %{http_code}\n"
done

# Test authenticated endpoint rate limit (requires valid token)
TOKEN="your-jwt-token"
for i in {1..6000}; do
  curl http://localhost:5001/api/auth/me \
    -H "Authorization: Bearer $TOKEN" \
    -w "\nStatus: %{http_code}\n"
done
```

## Deployment

### Redis Setup

**Development:**
```bash
# Install Redis locally
brew install redis  # macOS
sudo apt-get install redis  # Ubuntu

# Start Redis
redis-server

# Verify connection
redis-cli ping  # Should return "PONG"
```

**Production (Render.com example):**
1. Add Redis managed service to Render dashboard
2. Copy internal Redis URL
3. Add `REDIS_URL` environment variable to web service
4. Redeploy application

**Alternative providers:**
- Railway: Managed Redis add-on
- AWS ElastiCache
- DigitalOcean Managed Redis
- Upstash (serverless Redis)

### Environment Variables

Production `.env` should include:
```bash
REDIS_URL=redis://user:password@host:port/db
```

### Monitoring

**Metrics to track:**
- Rate limit rejection rate (429 responses per minute)
- Redis connection health (success/failure rate)
- Average response time impact
- Top rate-limited IPs/users

**Logging:**
- Log every 429 response with: IP/user, endpoint, remaining tokens
- Log Redis connection failures
- Log unusual rate limit patterns (potential attack indicators)

**Alerts:**
- High rate limit rejection rate (> 100/minute) - possible attack
- Redis connection failures
- Unusual spike in rate-limited IPs

### Documentation Updates

1. **README.md**
   - Add Redis to prerequisites
   - Document `REDIS_URL` environment variable
   - Add Redis setup instructions

2. **API Documentation**
   - Document rate limit tiers
   - Explain 429 responses and headers
   - Provide guidance on handling rate limits in clients

3. **Troubleshooting**
   - Common rate limit errors
   - Redis connection issues
   - Rate limit bypass for testing

## Security Considerations

### IP Spoofing Prevention

- Trust `X-Forwarded-For` only when behind trusted reverse proxy
- In production with load balancer, use leftmost IP in XFF chain
- Validate IP format before using as Redis key

### Redis Security

- Use Redis AUTH (password) in production
- Use TLS for Redis connections if available
- Restrict Redis network access to application servers only
- Regular Redis backups (though rate limit data is transient)

### Abuse Mitigation

- Monitor for distributed attacks (many IPs at lower rates)
- Consider adding permanent IP blocks for severe abuse
- Implement exponential backoff for repeated violations
- Alert on unusual patterns

## Future Enhancements

### Phase 2 (Optional)

- **Dynamic rate limits**: Adjust limits based on system load
- **User-specific overrides**: Allow VIP users higher limits
- **Geographic rate limits**: Different limits by region
- **Rate limit dashboard**: Admin UI to view top consumers
- **Webhooks**: Notify admins of potential attacks

### Phase 3 (Optional)

- **Machine learning**: Detect anomalous traffic patterns
- **IP reputation integration**: Use external threat intelligence
- **Challenge-response**: CAPTCHA for rate-limited users
- **API keys with quotas**: Separate limits per API key

## Success Metrics

- Zero successful brute-force attacks on auth endpoints
- <1% false positive rate (legitimate users rate-limited)
- <10ms latency impact per request
- 99.9% Redis availability
- Automatic mitigation of DDoS attempts

## Rollback Plan

If rate limiting causes issues:

1. **Increase limits**: Adjust capacity/refill rates in code
2. **Disable specific middleware**: Remove layer from problematic routes
3. **Emergency bypass**: Add environment flag to disable rate limiting entirely
4. **Fail-open already implemented**: Redis failures don't block traffic

Emergency bypass (add to rate limit middleware):
```rust
if config.rate_limit_disabled {
    return Ok(next.run(request).await);
}
```

## Open Questions

None - all design decisions finalized.

## References

- [Token Bucket Algorithm](https://en.wikipedia.org/wiki/Token_bucket)
- [Redis Connection Manager](https://docs.rs/redis/latest/redis/aio/struct.ConnectionManager.html)
- [Axum Middleware](https://docs.rs/axum/latest/axum/middleware/)
- [HTTP 429 Status Code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/429)
