-- Visual categories
CREATE TYPE visual_category AS ENUM ('TEAM', 'INFRA', 'EVENT', 'PROJECT', 'OTHER');

-- Visuals table
CREATE TABLE IF NOT EXISTS visuals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    category visual_category NOT NULL,
    src VARCHAR(500) NOT NULL,
    alt_text VARCHAR(500),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_visuals_category ON visuals(category);
CREATE INDEX IF NOT EXISTS idx_visuals_created_by ON visuals(created_by);

-- Update trigger
CREATE TRIGGER update_visuals_updated_at BEFORE UPDATE ON visuals
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
