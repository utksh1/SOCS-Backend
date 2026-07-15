# Email Verification & Password Reset System - Design Specification

**Date:** 2026-07-15  
**Status:** Approved  
**Author:** AI Assistant & Utkarsh

## Overview

This specification defines the email verification and password reset flows for the SOCS backend. The system provides secure, time-expiring tokens that allow users to verify their email addresses upon registration and reset forgotten passwords.

## Requirements Summary

### Email Verification
- **Soft verification**: Users can log in immediately but have limited access to critical features until verified
- **30-minute token expiration** for security
- **Rate limiting**: 3 verification email requests per hour per email address
- Verification email sent automatically upon registration
- Users can request resend if token expires

### Password Reset
- **30-minute token expiration** for security
- **Rate limiting**: 3 password reset requests per hour per email address
- After successful reset, users must log in manually (no automatic JWT issuance)
- Confirmation email sent after password change

### Security & Access Control
- **Critical actions blocked** for unverified users:
  - Changing email address
  - Deleting account
  - Accessing sensitive data/admin functions
- Non-critical actions allowed (viewing profile, browsing content, etc.)
- Generic error messages to prevent email enumeration attacks
- Single-use tokens (marked as used after successful verification)

### Token Management
- **Dual cleanup strategy**:
  - On-demand: Delete tokens immediately when used or when validation fails due to expiration
  - Scheduled: Daily background task removes tokens older than 7 days
- Cryptographically secure random tokens (32 bytes, 256-bit security)
- Tokens hashed with bcrypt before storage

## Architecture

### Database Schema

#### 1. Users Table Modification

```sql
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;
```

Tracks when a user verified their email. `NULL` means unverified.

#### 2. Verification Tokens Table

```sql
CREATE TABLE verification_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    token_type VARCHAR(50) NOT NULL, -- 'email_verification' or 'password_reset'
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_verification_tokens_user_id ON verification_tokens(user_id);
CREATE INDEX idx_verification_tokens_expires_at ON verification_tokens(expires_at);
CREATE INDEX idx_verification_tokens_token_type ON verification_tokens(token_type);
```

**Design decisions:**
- `token_hash` stores bcrypt hash of the token (never store plain tokens)
- `token_type` enum allows reusing table for both verification types
- `used_at` tracks consumption (NULL = unused, non-NULL = consumed)
- `expires_at` indexed for efficient cleanup queries
- Foreign key cascade ensures orphaned tokens are cleaned when users deleted

#### 3. Rate Limit Tracking Table

```sql
CREATE TABLE token_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    token_type VARCHAR(50) NOT NULL,
    attempt_count INT DEFAULT 1,
    window_start TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_rate_limits_email_type ON token_rate_limits(email, token_type);
CREATE INDEX idx_rate_limits_window_start ON token_rate_limits(window_start);
```

**Design decisions:**
- Track by email (not user_id) to rate-limit even before account exists
- Separate counters for email verification vs password reset
- 1-hour rolling window tracked via `window_start`
- Records cleaned up during scheduled maintenance

### API Endpoints

All endpoints added to the existing auth router (`src/routes/auth.rs`):

#### Email Verification Endpoints

**POST /auth/resend-verification**
- **Authentication**: Required (JWT)
- **Rate limit**: 3 per hour per email
- **Request body**: None (uses authenticated user's email)
- **Response**: 
  - 200: `{"success": true, "message": "Verification email sent"}`
  - 429: `{"error": "Too many requests. Please try again later."}`
  - 400: `{"error": "Email already verified"}`

**GET /auth/verify-email?token=<token>**
- **Authentication**: Not required
- **Query params**: `token` (string, 64-character hex)
- **Response**:
  - 200: `{"success": true, "message": "Email verified successfully"}`
  - 400: `{"error": "Invalid or expired token"}`
  - 410: `{"error": "Token already used"}`

#### Password Reset Endpoints

**POST /auth/forgot-password**
- **Authentication**: Not required
- **Rate limit**: 3 per hour per email
- **Request body**: `{"email": "user@example.com"}`
- **Response**: 
  - 200: `{"success": true, "message": "If that email exists, you'll receive a password reset link"}`
  - 429: `{"error": "Too many requests. Please try again later."}`

**POST /auth/reset-password**
- **Authentication**: Not required
- **Request body**: 
  ```json
  {
    "token": "64-char-hex-string",
    "new_password": "newSecurePassword123"
  }
  ```
- **Response**:
  - 200: `{"success": true, "message": "Password reset successfully. Please log in."}`
  - 400: `{"error": "Invalid or expired token"}`
  - 410: `{"error": "Token already used"}`

### Component Architecture

#### New Rust Modules

**1. `src/models/verification_token.rs`**

```rust
pub enum TokenType {
    EmailVerification,
    PasswordReset,
}

pub struct VerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub token_type: TokenType,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
```

**2. `src/repositories/verification_token_repository.rs`**

Core database operations:

- `create_token(pool, user_id, token_hash, token_type, expires_at) -> Result<VerificationToken>`
  - Inserts new token record
  
- `find_valid_token(pool, token_hash, token_type) -> Result<Option<VerificationToken>>`
  - Finds token matching hash and type
  - Filters: `used_at IS NULL AND expires_at > NOW()`
  
- `mark_as_used(pool, token_id) -> Result<()>`
  - Sets `used_at = NOW()` for the token
  
- `delete_user_tokens(pool, user_id, token_type) -> Result<()>`
  - Invalidates all of a user's tokens of specified type
  - Used when generating new token to invalidate previous ones
  
- `cleanup_expired_tokens(pool, older_than_days) -> Result<u64>`
  - Deletes tokens where `expires_at < NOW() - INTERVAL 'X days'`
  - Returns count of deleted tokens
  - Called by scheduled task

**3. `src/repositories/rate_limit_repository.rs`**

Rate limiting operations:

- `check_and_increment_rate_limit(pool, email, token_type, max_attempts, window_hours) -> Result<bool>`
  - Checks if user is within rate limit
  - If within limit, increments counter or creates new record
  - Returns `true` if allowed, `false` if rate limit exceeded
  
- `cleanup_old_rate_limits(pool, older_than_hours) -> Result<u64>`
  - Removes rate limit records older than specified hours
  - Called during scheduled maintenance

**4. `src/services/verification_service.rs`**

Business logic layer:

- `generate_secure_token() -> String`
  - Generates 32-byte cryptographically secure random token
  - Returns as 64-character hex string
  - Uses `rand::thread_rng()`
  
- `hash_token(token: &str) -> Result<String>`
  - Hashes token with bcrypt (same as passwords)
  - Returns bcrypt hash string
  
- `send_verification_email(pool, email_service, user) -> Result<()>`
  - Checks rate limit (3/hour)
  - Generates secure token
  - Hashes and stores token in database
  - Invalidates any existing verification tokens for user
  - Sends email via EmailService
  - Returns generic error messages for security
  
- `verify_email_token(pool, token_string) -> Result<Uuid>`
  - Retrieves token candidates from database
  - Uses bcrypt to compare token_string against stored hashes
  - Validates expiration and unused status
  - Marks token as used
  - Updates user's `email_verified_at` field
  - Returns user_id on success
  
- `send_password_reset_email(pool, email_service, email) -> Result<()>`
  - Checks rate limit (3/hour)
  - Looks up user by email (returns generic message if not found)
  - Generates secure token
  - Hashes and stores token
  - Invalidates any existing password reset tokens for user
  - Sends email via EmailService
  - Always returns success message (prevents email enumeration)
  
- `reset_password_with_token(pool, token_string, new_password) -> Result<()>`
  - Validates token (same logic as verify_email_token)
  - Hashes new password with bcrypt
  - Updates user's password
  - Marks token as used
  - Sends confirmation email
  - Returns success
  
- `cleanup_expired_tokens(pool) -> Result<()>`
  - Calls repository cleanup functions
  - Removes tokens older than 7 days
  - Removes rate limit records older than 24 hours
  - Logs cleanup statistics

**5. `src/services/email_service.rs` (extend existing)**

Add new email methods to existing `EmailService`:

- `send_email_verification(&self, to_email, name, verification_link) -> Result<()>`
  - Sends verification email with SOCS branding
  - 30-minute expiration notice
  
- `send_password_reset(&self, to_email, name, reset_link) -> Result<()>`
  - Sends password reset email with SOCS branding
  - 30-minute expiration notice
  
- `send_password_reset_confirmation(&self, to_email, name) -> Result<()>`
  - Sends confirmation after successful password change
  - Includes timestamp and security notice

**6. `src/middleware/verification.rs`** (new)

Middleware for protecting critical routes:

```rust
pub async fn require_verified_email(
    Extension(user): Extension<SafeUser>,
    request: Request,
    next: Next,
) -> Result<Response> {
    if user.email_verified_at.is_none() {
        return Err(ApiError::Forbidden(
            "Email verification required for this action".to_string()
        ));
    }
    Ok(next.run(request).await)
}
```

Applied to routes:
- Email change endpoint
- Account deletion endpoint
- Any other critical actions identified later

## Data Flow

### Email Verification Flow

1. **Registration**
   ```
   User submits registration form
   ↓
   auth_service::register() creates user with email_verified_at = NULL
   ↓
   verification_service::send_verification_email() called automatically
   ↓
   Token generated (32 bytes), hashed, stored with 30-min expiration
   ↓
   Email sent with link: https://socs.network/verify-email?token=<token>
   ↓
   User receives welcome email + verification link
   ```

2. **Verification**
   ```
   User clicks link in email
   ↓
   Frontend calls GET /auth/verify-email?token=<token>
   ↓
   verification_service::verify_email_token() validates token
   ↓
   Token matched via bcrypt, expiration checked
   ↓
   User's email_verified_at set to NOW()
   ↓
   Token marked as used
   ↓
   Frontend shows success message, user gains full access
   ```

3. **Resend Verification**
   ```
   User logs in (unverified)
   ↓
   Frontend detects email_verified_at is NULL
   ↓
   Shows banner with "Resend verification email" button
   ↓
   User clicks, calls POST /auth/resend-verification
   ↓
   Rate limit checked (max 3/hour)
   ↓
   Previous verification tokens invalidated
   ↓
   New token generated and emailed
   ```

### Password Reset Flow

1. **Request Reset**
   ```
   User clicks "Forgot password" on login page
   ↓
   Frontend shows form requesting email
   ↓
   Submits POST /auth/forgot-password with email
   ↓
   Rate limit checked (max 3/hour per email)
   ↓
   User looked up by email (silent fail if not found)
   ↓
   Token generated, hashed, stored with 30-min expiration
   ↓
   Previous password reset tokens invalidated
   ↓
   Email sent with link: https://socs.network/reset-password?token=<token>
   ↓
   Generic success message returned (prevents email enumeration)
   ```

2. **Reset Password**
   ```
   User clicks link in email
   ↓
   Frontend shows password reset form with token in URL
   ↓
   User enters new password, submits POST /auth/reset-password
   ↓
   Token validated (bcrypt match, expiration check, unused check)
   ↓
   New password hashed with bcrypt
   ↓
   User's password updated
   ↓
   Token marked as used
   ↓
   Confirmation email sent
   ↓
   Frontend redirects to login page with success message
   ```

### Token Cleanup Flow

**On-demand cleanup** (happens during normal operations):
```
User verifies email or resets password
↓
Token marked as used (used_at = NOW())
↓
No further action (kept for audit trail)

OR

Token validation fails due to expiration
↓
Token deleted immediately
↓
Error returned to user
```

**Scheduled cleanup** (daily background task):
```
Cron job triggers verification_service::cleanup_expired_tokens()
↓
Delete tokens where expires_at < NOW() - INTERVAL '7 days'
↓
Delete rate_limit records where window_start < NOW() - INTERVAL '24 hours'
↓
Log cleanup statistics (tokens deleted, rate limits cleaned)
```

## Security Considerations

### Token Security

1. **Generation**: 32-byte cryptographically secure random tokens using `rand::thread_rng()`
2. **Storage**: Tokens hashed with bcrypt before database storage (never store plaintext)
3. **Validation**: Constant-time bcrypt comparison prevents timing attacks
4. **Single-use**: Tokens marked as used after successful verification
5. **Expiration**: 30-minute lifetime limits exposure window
6. **Invalidation**: Previous tokens invalidated when new token requested

### Rate Limiting

- 3 attempts per hour per email per token type
- Tracked by email address (not user ID) to handle pre-registration scenarios
- Separate counters for email verification vs password reset
- Generic error message: "Too many requests. Please try again later."
- Rate limit records cleaned up after 24 hours

### Email Enumeration Prevention

**Problem**: Attackers can probe the system to discover registered email addresses.

**Solutions**:
1. **Generic responses**: 
   - "If that email exists, you'll receive a password reset link"
   - Never confirm or deny email existence
2. **Consistent timing**: Both existing and non-existing emails take similar time to respond
3. **Rate limiting**: Prevents mass enumeration attempts

### Access Control

**Unverified user restrictions**:
- Middleware checks `email_verified_at IS NOT NULL` on critical routes
- Returns `403 Forbidden` with message: "Email verification required for this action"
- Applied to:
  - Email change endpoint
  - Account deletion endpoint
  - Future critical actions as needed

**Allowed for unverified users**:
- Login and authentication
- View own profile
- Browse public content
- Request verification email resend

### Token Invalidation Scenarios

Tokens are invalidated when:
1. Successfully used (marked with `used_at` timestamp)
2. User requests new token of same type (all previous tokens deleted)
3. User account deleted (CASCADE foreign key)
4. Token expires and cleanup runs
5. Admin manually invalidates (future enhancement)

## Error Handling

### Token Validation Errors

- **Invalid token format**: `400 Bad Request - "Invalid token format"`
- **Token not found / expired**: `400 Bad Request - "Invalid or expired token"`
- **Token already used**: `410 Gone - "Token already used"`
- **Database error**: `500 Internal Server Error - "An error occurred"`

### Rate Limiting Errors

- **Too many attempts**: `429 Too Many Requests - "Too many requests. Please try again later."`
- Includes `Retry-After` header with seconds until window reset

### Email Sending Errors

- Logged internally but never exposed to user
- User always sees success message
- Prevents revealing system state to attackers

## Testing Considerations

### Unit Tests

- Token generation produces valid 64-character hex strings
- Bcrypt hashing and verification works correctly
- Rate limit logic correctly counts and resets
- Token expiration calculation is accurate
- Token invalidation works for all scenarios

### Integration Tests

- Full email verification flow (register → receive email → verify)
- Full password reset flow (request → receive email → reset → login)
- Rate limiting blocks after 3 attempts
- Expired tokens are rejected
- Used tokens cannot be reused
- Unverified users blocked from critical actions

### Security Tests

- Token enumeration attempts fail
- Email enumeration attempts fail
- Timing attacks don't reveal email existence
- Used tokens immediately invalidated
- Expired tokens cannot be used

## Configuration

New environment variables (add to `.env`):

```bash
# Frontend URL for generating verification links
FRONTEND_URL=https://socs.network

# Token settings (defaults shown)
VERIFICATION_TOKEN_EXPIRY_MINUTES=30
PASSWORD_RESET_TOKEN_EXPIRY_MINUTES=30
TOKEN_RATE_LIMIT_MAX_ATTEMPTS=3
TOKEN_RATE_LIMIT_WINDOW_HOURS=1

# Cleanup settings
TOKEN_CLEANUP_RETENTION_DAYS=7
RATE_LIMIT_CLEANUP_RETENTION_HOURS=24
```

## Migration Strategy

### Database Migration Order

1. Run migration to add `email_verified_at` column to users table
2. Run migration to create `verification_tokens` table with indexes
3. Run migration to create `token_rate_limits` table with indexes

### Existing User Handling

**Option A: Auto-verify existing users** (Recommended)
```sql
-- In migration: Set email_verified_at for all existing users
UPDATE users SET email_verified_at = created_at WHERE email_verified_at IS NULL;
```

**Option B: Require verification**
- Existing users remain unverified
- Prompt them to verify on next login
- May cause friction but ensures email validity

**Recommendation**: Use Option A. Assume existing users have valid emails since they've been using the system.

### Rollback Plan

If issues arise:
1. Remove middleware from critical routes (restore full access)
2. Disable verification email sending in registration flow
3. Drop new tables if necessary (no data loss for existing users)

## Future Enhancements

- **Email change flow**: Verify new email before updating
- **Admin token invalidation**: Allow admins to invalidate specific tokens
- **Token usage audit log**: Track who used which tokens when
- **SMS verification**: Alternative to email verification
- **2FA integration**: Build on token infrastructure for two-factor auth
- **Account recovery**: Multi-step recovery process for compromised accounts

## Implementation Checklist

- [ ] Create database migrations
- [ ] Implement `VerificationToken` model
- [ ] Implement `verification_token_repository.rs`
- [ ] Implement `rate_limit_repository.rs`
- [ ] Implement `verification_service.rs`
- [ ] Extend `email_service.rs` with new templates
- [ ] Create `verification.rs` middleware
- [ ] Add routes to `auth.rs`
- [ ] Update registration flow to send verification email
- [ ] Add environment variables to config
- [ ] Implement scheduled cleanup task
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Update API documentation
- [ ] Frontend integration (separate task)

## Conclusion

This design provides a secure, industry-standard email verification and password reset system that:
- Prevents email enumeration attacks
- Uses cryptographically secure tokens
- Implements proper rate limiting
- Maintains an audit trail
- Follows SOCS aesthetic for email templates
- Integrates cleanly with existing authentication system

The implementation focuses on security and user experience while maintaining the simplicity and clarity needed for reliable operation.
