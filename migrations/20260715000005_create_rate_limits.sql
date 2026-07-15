-- Create rate limiting table for verification token requests
CREATE TABLE token_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    token_type VARCHAR(50) NOT NULL CHECK (token_type IN ('email_verification', 'password_reset')),
    attempt_count INT NOT NULL DEFAULT 1,
    window_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Composite index for rate limit lookups
CREATE INDEX idx_rate_limits_email_type ON token_rate_limits(email, token_type);

-- Index for cleanup queries
CREATE INDEX idx_rate_limits_window_start ON token_rate_limits(window_start);

-- Unique constraint to prevent duplicate rate limit entries
CREATE UNIQUE INDEX idx_rate_limits_unique ON token_rate_limits(email, token_type, window_start);

-- Add comment for documentation
COMMENT ON TABLE token_rate_limits IS 'Tracks rate limits for verification email requests (3 per hour per email per type)';
COMMENT ON COLUMN token_rate_limits.window_start IS 'Start of the current 1-hour rate limit window';
