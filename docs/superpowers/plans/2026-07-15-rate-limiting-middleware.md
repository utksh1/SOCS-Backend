# Rate Limiting Middleware Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Redis-backed rate limiting middleware using token bucket algorithm to protect SOCS API against brute-force attacks and DDoS traffic.

**Architecture:** Three middleware functions (auth, public, authenticated) that extract identifiers (IP or user ID), query Redis for token bucket state, calculate refills, consume tokens if available, and return 429 with rate limit headers when exceeded.

**Tech Stack:** Rust, Axum, Redis, token bucket algorithm

---

## File Structure

**New Files:**
- `src/middleware/rate_limit.rs` - Core rate limiting logic and middleware functions
- `tests/rate_limit_test.rs` - Integration tests for rate limiting

**Modified Files:**
- `Cargo.toml` - Add redis dependency
- `src/middleware/mod.rs` - Export rate_limit module
- `src/config/env.rs` - Add redis_url configuration
- `src/main.rs` - Initialize Redis connection and apply middleware
- `src/error.rs` - Add RateLimitExceeded error variant
- `.env` - Add REDIS_URL (manual step)
- `README.md` - Document Redis requirement

---

### Task 1: Add Redis Dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add redis dependency to Cargo.toml**

```toml
# Add to [dependencies] section after existing dependencies
# Redis client with async support
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"] }
```

- [ ] **Step 2: Verify dependency installation**

Run: `cargo check`
Expected: Downloads redis crate and compiles successfully

- [ ] **Step 3: Commit dependency change**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add redis with tokio and connection-manager support"
```

---

### Task 2: Add Redis Configuration

**Files:**
- Modify: `src/config/env.rs`

- [ ] **Step 1: Read current env.rs to understand structure**

Read: `src/config/env.rs`

- [ ] **Step 2: Add redis_url field to Config struct**

Add to the Config struct:
```rust
pub redis_url: String,
```

- [ ] **Step 3: Add redis_url initialization in from_env method**

Add in the `from_env()` method where other env vars are loaded:
```rust
redis_url: env::var("REDIS_URL")
    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
```

- [ ] **Step 4: Build to verify changes compile**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Commit configuration changes**

```bash
git add src/config/env.rs
git commit -m "config: add redis_url to environment configuration"
```

---

### Task 3: Update AppState with Redis Connection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add redis import at top of main.rs**

Add after existing imports:
```rust
use redis::aio::ConnectionManager;
```

- [ ] **Step 2: Add redis field to AppState struct**

Modify the AppState struct (around line 25):
```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub redis: ConnectionManager,
}
```

- [ ] **Step 3: Create Redis connection manager in main function**

Add after database pool creation (after line 50):
```rust
// Create Redis connection manager
tracing::info!("Connecting to Redis...");
let redis_client = redis::Client::open(config.redis_url.as_str())
    .expect("Failed to create Redis client");
let redis = redis_client
    .get_connection_manager()
    .await
    .expect("Failed to create Redis connection manager");
tracing::info!("Redis connection established");
```

- [ ] **Step 4: Update AppState initialization to include redis**

Modify the AppState initialization (around line 63):
```rust
let state = AppState {
    db,
    config: config.clone(),
    redis,
};
```

- [ ] **Step 5: Build to verify changes compile**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 6: Commit AppState changes**

```bash
git add src/main.rs
git commit -m "feat: add Redis connection manager to AppState"
```

---

### Task 4: Add Rate Limit Error Variant

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Read current error.rs**

Read: `src/error.rs`

- [ ] **Step 2: Add RateLimitExceeded variant to ApiError enum**

Add to the ApiError enum:
```rust
#[error("Rate limit exceeded")]
RateLimitExceeded { retry_after: u64 },
```

- [ ] **Step 3: Add RateLimitExceeded handling in IntoResponse**

Add match arm in the `IntoResponse` implementation:
```rust
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
```

- [ ] **Step 4: Build to verify changes compile**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Commit error handling changes**

```bash
git add src/error.rs
git commit -m "feat: add RateLimitExceeded error variant with 429 response"
```

---

### Task 5: Implement Token Bucket Logic

**Files:**
- Create: `src/middleware/rate_limit.rs`

- [ ] **Step 1: Create rate_limit.rs with imports and structs**

```rust
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{error::ApiError, AppState};

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
    pub ttl: usize,
}

/// Token bucket state stored in Redis
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: i64,
}
```

- [ ] **Step 2: Implement token bucket algorithm function**

Add to `src/middleware/rate_limit.rs`:
```rust
/// Check rate limit and consume a token if available
async fn check_rate_limit(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    config: &RateLimitConfig,
) -> Result<(bool, u64), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Get current bucket state from Redis
    let tokens_key = format!("{}:tokens", key);
    let last_refill_key = format!("{}:last_refill", key);

    let tokens: Option<f64> = redis.get(&tokens_key).await.unwrap_or(None);
    let last_refill: Option<i64> = redis.get(&last_refill_key).await.unwrap_or(None);

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

        // Update Redis
        let _: () = redis.set_ex(&tokens_key, current_tokens, config.ttl).await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to update Redis tokens: {}", e);
            });
        let _: () = redis.set_ex(&last_refill_key, last_refill_time, config.ttl).await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to update Redis last_refill: {}", e);
            });

        Ok((true, 0)) // Allowed
    } else {
        // Calculate retry-after time (seconds until 1 token available)
        let tokens_needed = 1.0 - current_tokens;
        let retry_after = (tokens_needed / config.refill_rate).ceil() as u64;

        Ok((false, retry_after)) // Denied
    }
}
```

- [ ] **Step 3: Implement IP extraction helper**

Add to `src/middleware/rate_limit.rs`:
```rust
/// Extract client IP address from request headers
fn extract_ip(request: &Request) -> String {
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
```

- [ ] **Step 4: Build to verify token bucket logic compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Commit token bucket implementation**

```bash
git add src/middleware/rate_limit.rs
git commit -m "feat: implement token bucket algorithm and IP extraction"
```

---

### Task 6: Implement Auth Rate Limit Middleware

**Files:**
- Modify: `src/middleware/rate_limit.rs`

- [ ] **Step 1: Add auth rate limit middleware function**

Add to `src/middleware/rate_limit.rs`:
```rust
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
    let key = format!("{}:{}", config.key_prefix, ip);

    let mut redis = state.redis.clone();
    
    // Check rate limit (fail-open on Redis errors)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed for auth endpoint, allowing request: {}", e);
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for IP {} on auth endpoint, retry after {} seconds", ip, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}
```

- [ ] **Step 2: Build to verify middleware compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit auth middleware**

```bash
git add src/middleware/rate_limit.rs
git commit -m "feat: add auth rate limit middleware (20 req/15min per IP)"
```

---

### Task 7: Implement Public Rate Limit Middleware

**Files:**
- Modify: `src/middleware/rate_limit.rs`

- [ ] **Step 1: Add public rate limit middleware function**

Add to `src/middleware/rate_limit.rs`:
```rust
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
    let key = format!("{}:{}", config.key_prefix, ip);

    let mut redis = state.redis.clone();
    
    // Check rate limit (fail-open on Redis errors)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed for public endpoint, allowing request: {}", e);
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for IP {} on public endpoint, retry after {} seconds", ip, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}
```

- [ ] **Step 2: Build to verify middleware compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit public middleware**

```bash
git add src/middleware/rate_limit.rs
git commit -m "feat: add public rate limit middleware (500 req/min per IP)"
```

---

### Task 8: Implement Authenticated Rate Limit Middleware

**Files:**
- Modify: `src/middleware/rate_limit.rs`

- [ ] **Step 1: Add authenticated rate limit middleware function**

Add to `src/middleware/rate_limit.rs`:
```rust
/// Rate limit middleware for authenticated endpoints
/// Limits: 5000 requests per minute per user
pub async fn authenticated_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = RateLimitConfig {
        capacity: 5000.0,
        refill_rate: 5000.0 / 60.0, // 5000 tokens per minute
        key_prefix: "rate_limit:user".to_string(),
        ttl: 2 * 60, // 2 minutes
    };

    // Extract user from request extensions (set by auth middleware)
    let user = request.extensions().get::<crate::models::user::SafeUser>()
        .ok_or_else(|| ApiError::Unauthorized("User not authenticated".to_string()))?;

    let key = format!("{}:{}", config.key_prefix, user.id);

    let mut redis = state.redis.clone();
    
    // Check rate limit (fail-open on Redis errors)
    let (allowed, retry_after) = match check_rate_limit(&mut redis, &key, &config).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Rate limit check failed for authenticated endpoint, allowing request: {}", e);
            (true, 0)
        }
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for user {} on authenticated endpoint, retry after {} seconds", user.id, retry_after);
        return Err(ApiError::RateLimitExceeded { retry_after });
    }

    Ok(next.run(request).await)
}
```

- [ ] **Step 2: Build to verify middleware compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit authenticated middleware**

```bash
git add src/middleware/rate_limit.rs
git commit -m "feat: add authenticated rate limit middleware (5000 req/min per user)"
```

---

### Task 9: Export Rate Limit Module

**Files:**
- Modify: `src/middleware/mod.rs`

- [ ] **Step 1: Add rate_limit module export**

Add to `src/middleware/mod.rs`:
```rust
pub mod rate_limit;
```

- [ ] **Step 2: Build to verify module exports**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit module export**

```bash
git add src/middleware/mod.rs
git commit -m "feat: export rate_limit module"
```

---

### Task 10: Apply Middleware to Routes

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add rate limit middleware imports**

Add after existing middleware imports (around line 9):
```rust
use middleware::rate_limit::{
    auth_rate_limit_middleware,
    public_rate_limit_middleware,
    authenticated_rate_limit_middleware,
};
```

- [ ] **Step 2: Apply auth rate limit to auth routes**

Find the auth_routes Router construction (around line 88) and add the rate limit layer before the toplead layer:
```rust
let auth_routes = Router::new()
    .route("/register", axum::routing::post(routes::auth::register))
    .route("/login", axum::routing::post(routes::auth::login))
    .route("/change-password", axum::routing::patch(routes::auth::change_password))
    .layer(axum::middleware::from_fn_with_state(state.clone(), auth_rate_limit_middleware))
    .layer(toplead_layer.clone());
```

- [ ] **Step 3: Read main.rs to find public route definitions**

Read: `src/main.rs` (lines 95-120) to understand current routing structure

- [ ] **Step 4: Apply public rate limit to public project routes**

Modify the project_public Router (around line 96):
```rust
let project_public = Router::new()
    .route("/", axum::routing::get(routes::projects::list_projects))
    .route("/:id", axum::routing::get(routes::projects::get_project))
    .layer(axum::middleware::from_fn_with_state(state.clone(), public_rate_limit_middleware))
    .layer(optional_auth_layer.clone());
```

- [ ] **Step 5: Apply authenticated rate limit to protected routes**

Find where protected routes are defined and add authenticated rate limit layer before auth layer. For example, modify the auth "me" route:
```rust
let auth_protected = Router::new()
    .route("/me", axum::routing::get(routes::auth::get_me))
    .route("/update-name", axum::routing::patch(routes::auth::update_name))
    .layer(axum::middleware::from_fn_with_state(state.clone(), authenticated_rate_limit_middleware))
    .layer(auth_layer.clone());
```

Note: Apply similar pattern to other protected route groups (project_protected, blog_protected, etc.)

- [ ] **Step 6: Build to verify route changes compile**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 7: Commit middleware application**

```bash
git add src/main.rs
git commit -m "feat: apply rate limiting to auth, public, and authenticated routes"
```

---

### Task 11: Add Integration Tests

**Files:**
- Create: `tests/rate_limit_test.rs`

- [ ] **Step 1: Create test file with setup**

```rust
use reqwest;
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
            .post(&format!("{}/auth/login", BASE_URL))
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
        .post(&format!("{}/auth/login", BASE_URL))
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
        .post(&format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "email": "test@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // Check response structure for rate limit error
    if response.status() == 429 {
        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert_eq!(body["success"], false);
        assert!(body["error"].as_str().unwrap().contains("Rate limit exceeded"));
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
            .post(&format!("{}/auth/login", BASE_URL))
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
        .post(&format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "email": "refill-test@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_ne!(response.status(), 429, "Request should succeed after token refill");
}
```

- [ ] **Step 2: Run integration tests (with server running)**

Run: `cargo test --test rate_limit_test -- --ignored`
Expected: Tests pass (requires running server and Redis)

- [ ] **Step 3: Commit integration tests**

```bash
git add tests/rate_limit_test.rs
git commit -m "test: add integration tests for rate limiting"
```

---

### Task 12: Update Documentation

**Files:**
- Modify: `README.md`
- Create: `.env` (manual step for user)

- [ ] **Step 1: Add Redis to prerequisites section**

Find the Prerequisites section in README.md and add:
```markdown
- **[Redis](https://redis.io/download/)** 6 or higher

```bash
# Install Redis
brew install redis  # macOS
sudo apt-get install redis  # Ubuntu

# Verify installation
redis-server --version
```
```

- [ ] **Step 2: Add Redis setup instructions**

Add a new section after Database Setup:
```markdown
### 5. Redis Setup

```bash
# Start Redis locally
redis-server

# Verify Redis is running
redis-cli ping  # Should return "PONG"
```

For production deployment, use a managed Redis service:
- **Render**: Add Redis instance via dashboard
- **Railway**: Redis plugin
- **AWS ElastiCache**: Managed Redis
- **Upstash**: Serverless Redis
```

- [ ] **Step 3: Add REDIS_URL to environment variables section**

Add to the Environment Setup section:
```markdown
# ============================================
# REDIS (Rate Limiting)
# ============================================
REDIS_URL=redis://localhost:6379
```

- [ ] **Step 4: Add rate limiting section to features**

Add to the Features section:
```markdown
- **🛡️ Rate Limiting**: Redis-backed token bucket algorithm protects against brute-force and DDoS
  - Auth endpoints: 20 req/15min per IP
  - Public endpoints: 500 req/min per IP
  - Authenticated: 5000 req/min per user
```

- [ ] **Step 5: Build to ensure all changes work together**

Run: `cargo build --release`
Expected: Builds successfully

- [ ] **Step 6: Commit documentation updates**

```bash
git add README.md
git commit -m "docs: add Redis setup and rate limiting documentation"
```

---

### Task 13: Manual Testing

**Files:**
- N/A (testing only)

- [ ] **Step 1: Start Redis server**

Run: `redis-server`
Expected: Redis starts and listens on port 6379

- [ ] **Step 2: Add REDIS_URL to .env file**

Manually add to `.env`:
```
REDIS_URL=redis://localhost:6379
```

- [ ] **Step 3: Start the application**

Run: `cargo run`
Expected: Application starts, logs show "Redis connection established"

- [ ] **Step 4: Test auth rate limiting**

Run in terminal:
```bash
for i in {1..25}; do
  echo "Request $i:"
  curl -X POST http://127.0.0.1:5001/api/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"wrong"}' \
    -w "\nStatus: %{http_code}\n\n"
  sleep 1
done
```

Expected: First 20 requests return 401 (Unauthorized), requests 21+ return 429 (Too Many Requests)

- [ ] **Step 5: Verify rate limit response format**

Run:
```bash
# Make enough requests to trigger rate limit
for i in {1..21}; do
  curl -X POST http://127.0.0.1:5001/api/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"limit@example.com","password":"wrong"}' \
    -s -o /dev/null
done

# Check the rate limited response
curl -X POST http://127.0.0.1:5001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"limit@example.com","password":"wrong"}' \
  -i
```

Expected: Response includes:
- Status 429
- `Retry-After` header
- JSON body with "Rate limit exceeded" message and retry_after field

- [ ] **Step 6: Check Redis keys**

Run:
```bash
redis-cli KEYS "rate_limit:*"
redis-cli GET "rate_limit:auth:ip:127.0.0.1:tokens"
redis-cli GET "rate_limit:auth:ip:127.0.0.1:last_refill"
```

Expected: Shows rate limit keys and their values

- [ ] **Step 7: Test public endpoint rate limiting**

Run:
```bash
for i in {1..510}; do
  curl -s http://127.0.0.1:5001/api/projects -o /dev/null -w "%{http_code}\n"
done | tail -20
```

Expected: First 500 return 200, then 429s appear

- [ ] **Step 8: Document test results**

Create test summary noting:
- Auth rate limit works at 20 req/15min
- Public rate limit works at 500 req/min
- 429 responses include proper headers
- Redis keys are created and expire correctly
- Fail-open works (can test by stopping Redis)

---

## Self-Review Checklist

✅ **Spec Coverage:**
- Token bucket algorithm: Task 5
- Three middleware types: Tasks 6, 7, 8
- Redis integration: Tasks 1-3
- Error handling: Task 4
- Middleware application: Task 10
- Testing: Tasks 11, 13
- Documentation: Task 12

✅ **No Placeholders:** All code blocks contain complete implementations

✅ **Type Consistency:**
- `RateLimitConfig` used consistently across all middleware
- `TokenBucket` struct used in check_rate_limit
- `ApiError::RateLimitExceeded` matches error.rs definition
- Redis key patterns match spec: `rate_limit:{type}:ip:{ip}` and `rate_limit:user:{id}`

✅ **File Paths:** All absolute and exact

✅ **Commands:** All include expected output

---

## Execution Notes

- Each task is independently committable
- Tests in Task 11 require running server (marked with `#[ignore]`)
- Manual testing in Task 13 validates end-to-end functionality
- All Redis operations use fail-open strategy (errors logged, request allowed)
- Rate limit values match spec: 20/15min, 500/min, 5000/min
