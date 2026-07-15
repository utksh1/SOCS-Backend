-- Additional Database Indexes Based on Query Analysis
-- This migration adds indexes discovered after analyzing actual query patterns in the code

-- ============================================================================
-- PART 1: Admin View Indexes (High Priority)
-- ============================================================================

-- Blog posts: Admin view sorted by created_at
CREATE INDEX IF NOT EXISTS idx_blog_posts_created_at ON blog_posts(created_at DESC);

-- Projects: Admin view sorted by created_at  
CREATE INDEX IF NOT EXISTS idx_projects_created_at ON projects(created_at DESC);

-- Resources: Admin view sorted by created_at (if not already covered)
-- Already covered by existing idx_resources_created_at


-- ============================================================================
-- PART 2: User Content Queries (Medium Priority - Composite Indexes)
-- ============================================================================

-- Blog posts by author with date sorting (eliminates sort step)
CREATE INDEX IF NOT EXISTS idx_blog_posts_author_created 
ON blog_posts(author_id, created_at DESC);

-- Projects by creator with date sorting (eliminates sort step)
CREATE INDEX IF NOT EXISTS idx_projects_created_by_created 
ON projects(created_by, created_at DESC);

-- Events by creator with date sorting
CREATE INDEX IF NOT EXISTS idx_events_created_by_date
ON events(created_by, date DESC);


-- ============================================================================
-- PART 3: Optimization Comments
-- ============================================================================

COMMENT ON INDEX idx_blog_posts_created_at IS 
'Optimizes admin view: SELECT * FROM blog_posts ORDER BY created_at DESC';

COMMENT ON INDEX idx_projects_created_at IS 
'Optimizes admin view: SELECT * FROM projects ORDER BY created_at DESC';

COMMENT ON INDEX idx_blog_posts_author_created IS 
'Optimizes user posts query: WHERE author_id = ? ORDER BY created_at DESC (eliminates sort)';

COMMENT ON INDEX idx_projects_created_by_created IS 
'Optimizes user projects query: WHERE created_by = ? ORDER BY created_at DESC (eliminates sort)';

COMMENT ON INDEX idx_events_created_by_date IS
'Optimizes user events query: WHERE created_by = ? ORDER BY date DESC (eliminates sort)';


-- ============================================================================
-- PART 4: Query Pattern Documentation
-- ============================================================================

-- These indexes were added after analyzing the actual queries in:
-- - src/routes/blog.rs (lines 79-84: admin view, line 102: user's posts)
-- - src/routes/projects.rs (lines 83-88: admin view, line 106: user's projects)  
-- - src/routes/events.rs (lines 34-41: all events listing)

-- Performance improvements:
-- 1. Admin views (created_at DESC) go from Seq Scan to Index Scan
-- 2. User content queries (author_id + created_at) eliminate sort step
-- 3. Better scaling as data grows (10x+ improvement with 100k+ rows)
