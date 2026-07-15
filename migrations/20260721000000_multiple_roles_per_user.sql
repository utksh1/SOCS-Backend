-- Change user role from single enum to array of enums
-- Users can now have multiple roles (e.g., Member + Lead + Core)

-- Step 1: Add new roles array column as nullable first
ALTER TABLE users ADD COLUMN IF NOT EXISTS roles user_role[];

-- Step 2: Migrate existing single role to array
-- If role column exists, convert it to array format
DO $$ 
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'role'
    ) THEN
        -- Copy old role to new roles array
        UPDATE users 
        SET roles = ARRAY[role]::user_role[] 
        WHERE role IS NOT NULL;
        
        -- Drop the old single role column
        ALTER TABLE users DROP COLUMN role;
    END IF;
END $$;

-- Step 3: Ensure everyone has at least Member role
UPDATE users 
SET roles = ARRAY['MEMBER']::user_role[] 
WHERE roles IS NULL OR array_length(roles, 1) IS NULL;

-- Add Member role to users who don't have it
UPDATE users 
SET roles = array_append(roles, 'MEMBER'::user_role)
WHERE NOT ('MEMBER'::user_role = ANY(roles));

-- Step 4: Make roles column NOT NULL with default
ALTER TABLE users ALTER COLUMN roles SET NOT NULL;
ALTER TABLE users ALTER COLUMN roles SET DEFAULT ARRAY['MEMBER']::user_role[];

-- Step 5: Create index for faster role lookups
CREATE INDEX IF NOT EXISTS idx_users_roles ON users USING GIN(roles);
