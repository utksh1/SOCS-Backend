-- Remove ADMIN role from user_role enum
-- First, update existing admin users to MANAGEMENT
UPDATE users SET role = 'MANAGEMENT' WHERE role = 'ADMIN';

-- Create new enum without ADMIN
CREATE TYPE user_role_new AS ENUM ('MEMBER', 'MANAGEMENT');

-- Update the table to use new enum
ALTER TABLE users 
  ALTER COLUMN role TYPE user_role_new 
  USING role::text::user_role_new;

-- Drop old enum and rename new one
DROP TYPE user_role;
ALTER TYPE user_role_new RENAME TO user_role;
