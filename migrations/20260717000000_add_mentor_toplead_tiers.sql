-- Add new member tiers: toplead and mentor
-- New hierarchy: toplead > mentor > core > lead > member

ALTER TYPE member_tier ADD VALUE IF NOT EXISTS 'toplead';
ALTER TYPE member_tier ADD VALUE IF NOT EXISTS 'mentor';

-- Note: PostgreSQL doesn't allow reordering enum values
-- The order in code (Rust) will handle the hierarchy
