-- Complete cleanup: Remove all granular roles and keep only MEMBER and MANAGEMENT
-- Update all users to use only these two roles

-- First, migrate existing users to appropriate roles
UPDATE users SET role = 'MANAGEMENT' 
WHERE role IN ('ADMIN', 'EVENT_ORGANIZER', 'BLOG_EDITOR', 'RESOURCE_MANAGER', 'TEAM_LEAD');

UPDATE users SET role = 'MEMBER' 
WHERE role NOT IN ('MEMBER', 'MANAGEMENT');

-- Create new simplified enum with only MEMBER and MANAGEMENT
CREATE TYPE user_role_new AS ENUM ('MEMBER', 'MANAGEMENT');

-- Update the table to use new enum
ALTER TABLE users 
  ALTER COLUMN role TYPE user_role_new 
  USING role::text::user_role_new;

-- Drop old enum and rename new one
DROP TYPE user_role;
ALTER TYPE user_role_new RENAME TO user_role;

