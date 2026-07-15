-- The legacy `team` table was removed in 20260719000000.  Its CASCADE drop
-- also removed these tables because they referenced `team(id)`.  Recreate the
-- relationships against the canonical users table and make contributions
-- participate in the soft-delete lifecycle used by user_service.

CREATE TABLE IF NOT EXISTS project_contributors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    team_member_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    role VARCHAR(100) NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, team_member_id)
);

CREATE INDEX IF NOT EXISTS idx_project_contributors_project_id
    ON project_contributors(project_id);
CREATE INDEX IF NOT EXISTS idx_project_contributors_member_id
    ON project_contributors(team_member_id);

CREATE TABLE IF NOT EXISTS team_contributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_member_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    contribution_type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    url VARCHAR(500),
    contribution_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ DEFAULT NULL
);

ALTER TABLE team_contributions
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ DEFAULT NULL;

CREATE INDEX IF NOT EXISTS idx_team_contributions_member_id
    ON team_contributions(team_member_id);
CREATE INDEX IF NOT EXISTS idx_team_contributions_date
    ON team_contributions(contribution_date DESC);
CREATE INDEX IF NOT EXISTS idx_team_contributions_active_member_date
    ON team_contributions(team_member_id, contribution_date DESC)
    WHERE deleted_at IS NULL;

DROP TRIGGER IF EXISTS update_team_contributions_updated_at ON team_contributions;
CREATE TRIGGER update_team_contributions_updated_at
    BEFORE UPDATE ON team_contributions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
