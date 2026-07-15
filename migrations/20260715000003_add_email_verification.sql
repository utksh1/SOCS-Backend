-- Add email verification tracking to users table
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;

-- Set existing users as verified (grandfather clause)
UPDATE users SET email_verified_at = created_at WHERE email_verified_at IS NULL;

-- Add index for queries filtering by verification status
CREATE INDEX idx_users_email_verified_at ON users(email_verified_at) WHERE email_verified_at IS NULL;
