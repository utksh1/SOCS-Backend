-- Change user role from single enum to array of enums
-- Users can now have multiple roles (e.g., Member + Lead + Core)

-- Drop the old single role column
ALTER TABLE users DROP COLUMN IF EXISTS role;

-- Add new roles array column (everyone has at least Member)
ALTER TABLE users ADD COLUMN roles user_role[] NOT NULL DEFAULT ARRAY['MEMBER']::user_role[];

-- Create index for faster role lookups
CREATE INDEX IF NOT EXISTS idx_users_roles ON users USING GIN(roles);

-- Update existing users to have Member role
UPDATE users SET roles = ARRAY['MEMBER']::user_role[] WHERE roles IS NULL OR array_length(roles, 1) IS NULL;
