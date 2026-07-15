-- Phase 5: Rich Content Data Models
-- This migration adds dynamic content for project features, event timelines, 
-- team contributions, and event registrations

-- Project Features (replaces hardcoded features)
CREATE TABLE IF NOT EXISTS project_features (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_project_features_project_id ON project_features(project_id);
CREATE INDEX idx_project_features_order ON project_features(display_order);

-- Project Contributors (many-to-many with team members)
CREATE TABLE IF NOT EXISTS project_contributors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    team_member_id UUID NOT NULL REFERENCES team(id) ON DELETE CASCADE,
    role VARCHAR(100) NOT NULL, -- Lead, Developer, Designer, etc.
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, team_member_id)
);

CREATE INDEX idx_project_contributors_project_id ON project_contributors(project_id);
CREATE INDEX idx_project_contributors_member_id ON project_contributors(team_member_id);

-- Event Timeline Items (replaces hardcoded timeline)
CREATE TABLE IF NOT EXISTS event_timeline_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    time VARCHAR(10) NOT NULL, -- "09:00", "14:30"
    title VARCHAR(255) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_event_timeline_event_id ON event_timeline_items(event_id);
CREATE INDEX idx_event_timeline_order ON event_timeline_items(display_order);

-- Event Prerequisites (replaces hardcoded prerequisites)
CREATE TABLE IF NOT EXISTS event_prerequisites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_event_prerequisites_event_id ON event_prerequisites(event_id);
CREATE INDEX idx_event_prerequisites_order ON event_prerequisites(display_order);

-- Team Member Contributions (replaces hardcoded contribution log)
CREATE TABLE IF NOT EXISTS team_contributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_member_id UUID NOT NULL REFERENCES team(id) ON DELETE CASCADE,
    contribution_type VARCHAR(50) NOT NULL, -- PROJECT_CREATED, EVENT_LED, WRITEUP_PUBLISHED, etc.
    title VARCHAR(255) NOT NULL,
    description TEXT,
    url VARCHAR(500), -- Link to project/event/writeup
    contribution_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_team_contributions_member_id ON team_contributions(team_member_id);
CREATE INDEX idx_team_contributions_date ON team_contributions(contribution_date DESC);
CREATE INDEX idx_team_contributions_type ON team_contributions(contribution_type);

-- Update triggers for updated_at
CREATE TRIGGER update_project_features_updated_at BEFORE UPDATE ON project_features
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_timeline_items_updated_at BEFORE UPDATE ON event_timeline_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_prerequisites_updated_at BEFORE UPDATE ON event_prerequisites
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_team_contributions_updated_at BEFORE UPDATE ON team_contributions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
