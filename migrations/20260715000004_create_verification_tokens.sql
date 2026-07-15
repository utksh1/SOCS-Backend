-- Create verification tokens table for email verification and password reset
CREATE TABLE verification_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    token_type VARCHAR(50) NOT NULL CHECK (token_type IN ('email_verification', 'password_reset')),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_verification_tokens_user_id ON verification_tokens(user_id);
CREATE INDEX idx_verification_tokens_expires_at ON verification_tokens(expires_at);
CREATE INDEX idx_verification_tokens_token_type ON verification_tokens(token_type);
CREATE INDEX idx_verification_tokens_user_type ON verification_tokens(user_id, token_type);

-- Index for cleanup queries
CREATE INDEX idx_verification_tokens_cleanup ON verification_tokens(expires_at, used_at);

-- Index for token verification lookups
CREATE INDEX idx_verification_tokens_token_hash ON verification_tokens(token_hash);

-- Unique constraint to prevent duplicate token hashes (defensive security measure)
CREATE UNIQUE INDEX idx_verification_tokens_token_hash_unique ON verification_tokens(token_hash);

-- Add comment for documentation
COMMENT ON TABLE verification_tokens IS 'Stores hashed tokens for email verification and password reset flows';
COMMENT ON COLUMN verification_tokens.token_hash IS 'Bcrypt hash of the verification token (never store plaintext)';
COMMENT ON COLUMN verification_tokens.used_at IS 'Timestamp when token was used (NULL = unused)';
