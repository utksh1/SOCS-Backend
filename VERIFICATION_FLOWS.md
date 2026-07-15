# Email Verification & Password Reset Flows

This document describes the email verification and password reset functionality implemented in the SOCS backend.

## Overview

The system implements secure, time-expiring token-based flows for:
- **Email Verification**: Users must verify their email address after registration
- **Password Reset**: Users can securely reset forgotten passwords

## Architecture

- **Token Storage**: Database-backed with bcrypt-hashed tokens
- **Expiration**: 30-minute token lifetime
- **Rate Limiting**: 3 requests per hour per email per flow type
- **Security**: Soft verification (unverified users can login but have restricted access)

## API Endpoints

### Email Verification

#### 1. POST /api/auth/resend-verification
**Authentication Required**: Yes (JWT token)

Resends verification email to authenticated user.

**Response (200)**:
```json
{
  "success": true,
  "message": "Verification email sent. Please check your inbox."
}
```

**Error (400)** - Already verified:
```json
{
  "success": false,
  "message": "Email already verified"
}
```

**Error (429)** - Rate limit exceeded:
```json
{
  "success": false,
  "message": "Too many requests. Please try again later."
}
```

#### 2. GET /api/auth/verify-email?token={token}
**Authentication Required**: No

Verifies email address with the provided token.

**Query Parameters**:
- `token`: 64-character hex string

**Response (200)**:
```json
{
  "success": true,
  "message": "Email verified successfully!"
}
```

**Error (400)** - Invalid token:
```json
{
  "success": false,
  "message": "Invalid or expired token"
}
```

**Error (410)** - Token already used:
```json
{
  "success": false,
  "message": "Token already used"
}
```

### Password Reset

#### 3. POST /api/auth/forgot-password
**Authentication Required**: No

Initiates password reset flow by sending reset email.

**Request Body**:
```json
{
  "email": "user@example.com"
}
```

**Response (200)** - Always returns success to prevent email enumeration:
```json
{
  "success": true,
  "message": "If that email exists, you'll receive a password reset link shortly."
}
```

**Error (429)** - Rate limit exceeded:
```json
{
  "success": false,
  "message": "Too many requests. Please try again later."
}
```

#### 4. POST /api/auth/reset-password
**Authentication Required**: No

Resets password using the provided token.

**Request Body**:
```json
{
  "token": "64-character-hex-string",
  "new_password": "newSecurePassword123"
}
```

**Response (200)**:
```json
{
  "success": true,
  "message": "Password reset successfully. You can now log in with your new password."
}
```

**Error (400)** - Invalid token or weak password:
```json
{
  "success": false,
  "message": "Invalid or expired token"
}
```

## User Flows

### Registration Flow

1. User submits registration form → POST /api/auth/register
2. Backend creates user account with `email_verified_at = NULL`
3. Backend automatically sends verification email
4. User receives email with verification link: `https://socs.network/verify-email?token={token}`
5. User clicks link → Frontend calls GET /api/auth/verify-email?token={token}
6. Backend verifies token and sets `email_verified_at = NOW()`
7. User can now access all features

**Note**: Registration succeeds even if email sending fails (graceful degradation)

### Email Verification Resend Flow

1. User logs in with unverified email
2. Frontend shows "Please verify your email" message
3. User clicks "Resend verification email"
4. Frontend calls POST /api/auth/resend-verification (with JWT token)
5. User receives new verification email
6. User clicks link to verify

### Password Reset Flow

1. User clicks "Forgot password" on login page
2. User enters email → Frontend calls POST /api/auth/forgot-password
3. If email exists, user receives reset email with link: `https://socs.network/reset-password?token={token}`
4. User clicks link and enters new password
5. Frontend calls POST /api/auth/reset-password with token and new password
6. Backend validates token, updates password, sends confirmation email
7. User can now login with new password

## Protected Routes

Routes that require email verification (return 403 if unverified):

- **DELETE /api/users/:id** - User account deletion
- **PATCH /api/auth/change-password** - Password changes

## Security Features

### Token Security
- **32-byte cryptographic random**: Tokens generated using `rand::thread_rng()`
- **Bcrypt hashing**: Tokens stored as bcrypt hashes (cost 12)
- **Single-use**: Tokens marked as used after consumption
- **Time-limited**: 30-minute expiration
- **Old token invalidation**: New tokens invalidate previous ones

### Rate Limiting
- **3 requests per hour** per email per flow type
- Separate limits for email verification vs password reset
- Implemented with atomic database operations (no race conditions)
- Rolling window approach

### Email Enumeration Prevention
- Forgot password always returns success message
- Generic error messages ("Invalid or expired token")
- No distinction between "email not found" and "rate limited"

### Database Cleanup
- **Daily scheduled task**: Removes expired tokens (>7 days old)
- **Rate limit cleanup**: Removes old rate limit records (>24 hours)
- Non-blocking background task

## Configuration

Required environment variables:

```bash
# Frontend URL for generating verification links
FRONTEND_URL=https://socs.network

# Email service configuration (already configured)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=noreply@socs.network
SMTP_FROM_NAME=SOCS Team
```

## Database Schema

### users table
- `email_verified_at`: TIMESTAMPTZ - NULL if unverified, timestamp if verified

### verification_tokens table
- `id`: UUID primary key
- `user_id`: UUID foreign key to users
- `token_hash`: VARCHAR(60) - Bcrypt hash of token
- `token_type`: VARCHAR(50) - 'email_verification' or 'password_reset'
- `expires_at`: TIMESTAMPTZ - Token expiration time
- `used_at`: TIMESTAMPTZ - NULL if unused, timestamp when used
- `created_at`: TIMESTAMPTZ - Creation timestamp

### token_rate_limits table
- `id`: UUID primary key
- `email`: VARCHAR(255) - Email being rate limited
- `token_type`: VARCHAR(50) - Type of request
- `attempt_count`: INT - Number of attempts in window
- `window_start`: TIMESTAMPTZ - Start of rate limit window
- `created_at`: TIMESTAMPTZ - Creation timestamp

## Testing

### Manual Testing Checklist

**Email Verification:**
- [ ] New user registration sends verification email
- [ ] Verification link works and marks email as verified
- [ ] Resend verification works for unverified users
- [ ] Already verified users get appropriate message
- [ ] Rate limiting blocks excessive resend requests
- [ ] Expired tokens are rejected
- [ ] Used tokens are rejected

**Password Reset:**
- [ ] Forgot password sends reset email for existing users
- [ ] Reset link works and updates password
- [ ] Can login with new password after reset
- [ ] Confirmation email is sent after successful reset
- [ ] Rate limiting blocks excessive reset requests
- [ ] Expired reset tokens are rejected
- [ ] Used reset tokens are rejected
- [ ] Non-existent email returns generic success message (no enumeration)

**Protected Routes:**
- [ ] Unverified users cannot delete their account
- [ ] Unverified users cannot change their password
- [ ] Verified users can access all protected routes

**Edge Cases:**
- [ ] Email service failure doesn't break registration
- [ ] Concurrent requests respect rate limits (no race conditions)
- [ ] Token cleanup runs daily without errors
- [ ] Invalid token formats are rejected gracefully

## Troubleshooting

### User not receiving emails
1. Check SMTP configuration in environment variables
2. Check server logs for email sending errors
3. Verify email service credentials
4. Check spam folder

### "Token already used" error
- User clicked the link multiple times
- Solution: Use "Resend verification" or "Forgot password" to get a new token

### "Invalid or expired token" error
- Token expired (>30 minutes old)
- Token was never valid
- Solution: Request a new token

### Rate limit errors
- User exceeded 3 requests per hour
- Solution: Wait and try again after the hour window

## Implementation Details

### Repository Layer
- `verification_token_repository.rs`: Token CRUD operations
- `rate_limit_repository.rs`: Atomic rate limiting with upsert

### Service Layer
- `verification_service.rs`: Business logic for all verification flows
- `email_service.rs`: Email template rendering and sending

### Middleware
- `verification.rs`: Protects routes requiring verified email

### Background Tasks
- `tasks/cleanup.rs`: Daily cleanup of expired tokens and rate limits

## Future Enhancements

Potential improvements for future iterations:

1. **Email Templates**: Add more sophisticated HTML/CSS styling
2. **SMS Verification**: Add phone number verification as alternative
3. **Magic Links**: Implement passwordless authentication
4. **Token Analytics**: Track verification completion rates
5. **Custom Rate Limits**: Per-user or per-IP rate limiting
6. **Email Queue**: Retry failed emails with exponential backoff
7. **Audit Trail**: Log all verification attempts for security monitoring
