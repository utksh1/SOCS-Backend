-- Team member tiers
CREATE TYPE member_tier AS ENUM ('core', 'lead', 'member');

-- Team table
CREATE TABLE IF NOT EXISTS team (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(255) NOT NULL,
    skills TEXT[] NOT NULL DEFAULT '{}',
    tier member_tier NOT NULL DEFAULT 'member',
    github VARCHAR(500),
    linkedin VARCHAR(500),
    avatar_url VARCHAR(500),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_team_slug ON team(slug);
CREATE INDEX IF NOT EXISTS idx_team_tier ON team(tier);

-- Update trigger
CREATE TRIGGER update_team_updated_at BEFORE UPDATE ON team
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
