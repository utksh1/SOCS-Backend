-- Consolidate team table into users table with proper role hierarchy
-- Replace team tiers with user roles

-- Step 1: Create new user_role enum with the 5-tier hierarchy
CREATE TYPE user_role_new AS ENUM ('TOPLEAD', 'MENTOR', 'CORE', 'LEAD', 'MEMBER');

-- Step 2: Add new columns to users table from team table
ALTER TABLE users ADD COLUMN IF NOT EXISTS slug VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS position VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS skills TEXT[] DEFAULT '{}';
ALTER TABLE users ADD COLUMN IF NOT EXISTS github VARCHAR(500);
ALTER TABLE users ADD COLUMN IF NOT EXISTS linkedin VARCHAR(500);
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url VARCHAR(500);
ALTER TABLE users ADD COLUMN role_new user_role_new;

-- Step 3: Migrate data from team table to users table
UPDATE users u
SET 
    slug = t.slug,
    position = t.role,
    bio = t.bio,
    skills = t.skills,
    github = t.github,
    linkedin = t.linkedin,
    avatar_url = t.avatar_url
FROM team t
WHERE u.id = t.created_by;

-- Step 4: Update user roles based on team tiers
UPDATE users u
SET role_new = CASE 
    WHEN t.tier = 'toplead' THEN 'TOPLEAD'::user_role_new
    WHEN t.tier = 'mentor' THEN 'MENTOR'::user_role_new
    WHEN t.tier = 'core' THEN 'CORE'::user_role_new
    WHEN t.tier = 'lead' THEN 'LEAD'::user_role_new
    ELSE 'MEMBER'::user_role_new
END
FROM team t
WHERE u.id = t.created_by;

-- Step 5: Set default role for users not in team table
UPDATE users SET role_new = 'MEMBER'::user_role_new 
WHERE role_new IS NULL;

-- Step 6: Switch columns
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users RENAME COLUMN role_new TO role;

-- Step 7: Drop old enum and rename new one
DROP TYPE user_role;
ALTER TYPE user_role_new RENAME TO user_role;

ALTER TABLE users ALTER COLUMN role SET DEFAULT 'MEMBER'::user_role;
ALTER TABLE users ALTER COLUMN role SET NOT NULL;

-- Step 8: Add constraints and indexes
ALTER TABLE users ADD CONSTRAINT unique_user_slug UNIQUE (slug);
CREATE INDEX IF NOT EXISTS idx_users_slug ON users(slug);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- Step 9: Drop team table (data is now in users)
DROP TABLE IF EXISTS team CASCADE;
