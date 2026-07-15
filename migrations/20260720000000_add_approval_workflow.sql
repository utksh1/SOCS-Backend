-- Add approval status for content
CREATE TYPE content_status AS ENUM ('pending', 'approved', 'rejected');

-- Add status to projects table
ALTER TABLE projects ADD COLUMN status content_status NOT NULL DEFAULT 'pending';
ALTER TABLE projects ADD COLUMN approved_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN approved_at TIMESTAMPTZ;

-- Add index for project status
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

-- Add status to resources table
ALTER TABLE resources ADD COLUMN status content_status NOT NULL DEFAULT 'pending';
ALTER TABLE resources ADD COLUMN approved_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE resources ADD COLUMN approved_at TIMESTAMPTZ;

-- Add index for resource status
CREATE INDEX IF NOT EXISTS idx_resources_status ON resources(status);

-- Update blog_posts to use content_status instead of post_status
-- First, add the new column
ALTER TABLE blog_posts ADD COLUMN approval_status content_status NOT NULL DEFAULT 'pending';
ALTER TABLE blog_posts ADD COLUMN approved_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE blog_posts ADD COLUMN approved_at TIMESTAMPTZ;

-- Map existing post_status to approval_status
-- PUBLISHED -> approved, DRAFT -> pending, ARCHIVED -> rejected
UPDATE blog_posts SET approval_status = 'approved' WHERE status = 'PUBLISHED';
UPDATE blog_posts SET approval_status = 'pending' WHERE status = 'DRAFT';
UPDATE blog_posts SET approval_status = 'rejected' WHERE status = 'ARCHIVED';

-- Add index for blog approval status
CREATE INDEX IF NOT EXISTS idx_blog_posts_approval_status ON blog_posts(approval_status);

-- Project collaborators table
CREATE TABLE IF NOT EXISTS project_collaborators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_collaborators_project ON project_collaborators(project_id);
CREATE INDEX IF NOT EXISTS idx_project_collaborators_user ON project_collaborators(user_id);

-- Blog collaborators table
CREATE TABLE IF NOT EXISTS blog_collaborators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    blog_post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(blog_post_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_blog_collaborators_blog ON blog_collaborators(blog_post_id);
CREATE INDEX IF NOT EXISTS idx_blog_collaborators_user ON blog_collaborators(user_id);

-- Resource collaborators table
CREATE TABLE IF NOT EXISTS resource_collaborators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(resource_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_resource_collaborators_resource ON resource_collaborators(resource_id);
CREATE INDEX IF NOT EXISTS idx_resource_collaborators_user ON resource_collaborators(user_id);
