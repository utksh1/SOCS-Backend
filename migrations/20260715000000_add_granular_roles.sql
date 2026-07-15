-- Add new user roles for granular permissions
-- This migration extends the user_role enum with additional specialized roles

-- First, add the new role values to the enum
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'EVENT_ORGANIZER';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'BLOG_EDITOR';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'RESOURCE_MANAGER';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'TEAM_LEAD';

-- Note: The role hierarchy is:
-- MEMBER < EVENT_ORGANIZER/BLOG_EDITOR/RESOURCE_MANAGER/TEAM_LEAD < MANAGEMENT < ADMIN
-- 
-- Permissions by role:
-- MEMBER: View public content only
-- EVENT_ORGANIZER: Create/edit events, view event registrations
-- BLOG_EDITOR: Create/edit/delete blog posts
-- RESOURCE_MANAGER: Create/edit/delete resources
-- TEAM_LEAD: Manage team members directory
-- MANAGEMENT: All above permissions combined
-- ADMIN: Full system access including user management
