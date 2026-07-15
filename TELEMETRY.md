# Enhanced Logging & Telemetry

## Overview

The SOCS backend now includes comprehensive logging and telemetry using the `tracing` crate. This provides detailed insights into SQL failures, request paths, execution times, and system behavior for faster debugging in production.

## Features

### 1. Environment-Aware Logging

The application automatically configures logging based on the `ENVIRONMENT` variable:

- **Development**: Pretty console output with colors, line numbers, and thread IDs
- **Production**: JSON structured logs for easy parsing by log aggregators (e.g., ELK, Datadog, Splunk)

### 2. Structured Error Logging

All database errors are now logged with detailed context:

```rust
// Example log output for a constraint violation:
{
  "level": "ERROR",
  "error_code": "23505",
  "error_message": "duplicate key value violates unique constraint \"users_email_key\"",
  "constraint": "users_email_key",
  "table": "users"
}
```

**Error Categories Logged:**
- Database constraint violations (unique, foreign key, check constraints)
- Row not found errors
- Column not found errors
- Connection pool timeouts
- Connection pool closed errors
- Generic SQL errors

### 3. Request Logging Middleware

Every HTTP request is logged with:
- HTTP method and path
- Query parameters
- Client IP address (from X-Forwarded-For or X-Real-IP headers)
- Response status code
- Request duration in milliseconds
- Log level based on status code (info for 2xx, warn for 4xx, error for 5xx)

**Example log:**
```json
{
  "level": "INFO",
  "method": "POST",
  "path": "/api/blog",
  "status": 201,
  "duration_ms": 45,
  "client_ip": "192.168.1.100",
  "message": "Request completed successfully"
}
```

### 4. Authentication Logging

The auth middleware logs:
- Authentication attempts (success/failure)
- Token verification failures with reasons
- User ID and email on successful authentication
- User roles for authorization checks
- Request completion with timing

### 5. Repository Function Instrumentation

Database operations are instrumented with the `#[tracing::instrument]` macro:

**Example for user creation:**
```rust
#[tracing::instrument(name = "create_user", skip(pool, password_hash), fields(name = %name, email = %email))]
pub async fn create(pool: &PgPool, name: &str, email: &str, password_hash: &str) -> Result<User, sqlx::Error>
```

**Logs produced:**
- Function entry with parameters (sensitive data like passwords are skipped)
- Success: User ID and email
- Failure: Detailed error information
- Execution span for performance tracking

### 6. Route Handler Instrumentation

Key route handlers include detailed logging:

**Blog post creation:**
- User who created the post (ID and email)
- Generated slug
- Approval status
- Post status (draft/published)
- Success/failure with post ID

**Blog post deletion:**
- User requesting deletion
- Permission check results
- Deletion success confirmation

## Configuration

### Environment Variables

Add to your `.env` file:

```bash
# Logging configuration
ENVIRONMENT=development  # or "production"
RUST_LOG=socs_backend=debug,tower_http=debug,sqlx=debug
```

### Log Levels

- `error`: Critical failures, SQL errors, server errors (5xx)
- `warn`: Client errors (4xx), authentication failures, missing resources
- `info`: Successful operations, request completions
- `debug`: Detailed operation traces, query results
- `trace`: Very verbose, typically not used

### Production Recommendations

For production deployments:

1. Set `ENVIRONMENT=production` for JSON logging
2. Use `RUST_LOG=socs_backend=info,tower_http=info,sqlx=warn` to reduce verbosity
3. Configure log aggregation (e.g., ship to ELK stack, Datadog, CloudWatch)
4. Set up alerts on error-level logs
5. Monitor request duration metrics

## Usage Examples

### Viewing Logs in Development

```bash
# Start the server
cargo run

# Logs will be pretty-printed to console with colors
```

### Viewing Logs in Production

```bash
# Logs are in JSON format
ENVIRONMENT=production cargo run

# Example output:
{"timestamp":"2026-07-15T10:41:00.123Z","level":"INFO","fields":{"method":"GET","path":"/api/blog","status":200,"duration_ms":23},"message":"Request completed successfully"}
```

### Filtering Logs

```bash
# Only show errors
RUST_LOG=error cargo run

# Show info for your app, debug for SQL
RUST_LOG=socs_backend=info,sqlx=debug cargo run

# Trace everything (very verbose)
RUST_LOG=trace cargo run
```

## Debugging Production Issues

### Finding SQL Errors

Look for logs with `level: "ERROR"` and fields like `error_code`, `constraint`, or `table`:

```json
{
  "level": "ERROR",
  "error_code": "23503",
  "constraint": "blog_posts_author_id_fkey",
  "table": "blog_posts",
  "message": "Database constraint violation or SQL error"
}
```

### Tracking Slow Requests

Filter logs by `duration_ms` field:

```bash
# Using jq to find requests over 1000ms
cat logs.json | jq 'select(.duration_ms > 1000)'
```

### Identifying Failed Authentication

Look for auth-related warnings:

```json
{
  "level": "WARN",
  "method": "GET",
  "path": "/api/blog/my",
  "message": "Authentication failed: Invalid or expired token"
}
```

### Tracing User Actions

Use the `user_id` field to track all actions by a specific user:

```bash
# Filter logs for a specific user
cat logs.json | jq 'select(.user_id == "123e4567-e89b-12d3-a456-426614174000")'
```

## Performance Impact

The telemetry system is designed to have minimal performance impact:

- Structured logging avoids string formatting unless the log is actually emitted
- The `#[tracing::instrument]` macro uses compile-time code generation
- JSON serialization only occurs for logs that pass the filter level
- Request logging middleware uses `Instant` for high-precision, low-overhead timing

Typical overhead: **< 1ms per request** in production with info-level logging.

## Future Enhancements

Potential improvements:
- Distributed tracing with OpenTelemetry integration
- Metrics export (Prometheus format)
- Custom spans for complex business logic
- Correlation IDs across microservices
- Automatic performance profiling for slow requests
- Integration with APM tools (New Relic, DataDog APM)
