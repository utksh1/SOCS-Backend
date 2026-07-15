-- Existing indexes predate soft deletion. These partial indexes match the
-- public/admin list queries so deleted rows neither leak nor remain in the
-- hot portion of the index.

CREATE INDEX IF NOT EXISTS idx_projects_public_active_created
    ON projects(created_at DESC)
    WHERE status = 'approved' AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_creator_active_created
    ON projects(created_by, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_events_active_date
    ON events(date DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_blog_posts_public_active_published
    ON blog_posts(published_at DESC)
    WHERE status = 'PUBLISHED' AND approval_status = 'approved' AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_blog_posts_author_active_created
    ON blog_posts(author_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_resources_public_active_created
    ON resources(created_at DESC)
    WHERE status = 'approved' AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_resources_creator_active_created
    ON resources(created_by, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_users_active_created
    ON users(created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_event_registrations_active_event
    ON event_registrations(event_id)
    WHERE deleted_at IS NULL;
