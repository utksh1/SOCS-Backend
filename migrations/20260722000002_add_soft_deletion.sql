-- Add deleted_at column to primary entity tables
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE projects ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE events ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE blog_posts ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE resources ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE announcements ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE visuals ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE contacts ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE applications ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE event_registrations ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE notifications ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;

-- Add deleted_at to junction tables that cascade
ALTER TABLE project_features ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE event_timeline_items ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE event_prerequisites ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;

-- Create partial indexes for fast active record queries
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_projects_deleted_at ON projects(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_events_deleted_at ON events(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_blog_posts_deleted_at ON blog_posts(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_resources_deleted_at ON resources(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_announcements_deleted_at ON announcements(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_visuals_deleted_at ON visuals(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_contacts_deleted_at ON contacts(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_applications_deleted_at ON applications(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_event_registrations_deleted_at ON event_registrations(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_notifications_deleted_at ON notifications(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_project_features_deleted_at ON project_features(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_event_timeline_items_deleted_at ON event_timeline_items(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_event_prerequisites_deleted_at ON event_prerequisites(deleted_at) WHERE deleted_at IS NULL;
