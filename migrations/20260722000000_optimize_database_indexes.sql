-- Database Indexing & Query Optimization Migration
-- This migration adds missing indexes and composite indexes for common query patterns

-- ============================================================================
-- PART 1: Missing Foreign Key Indexes
-- ============================================================================

-- Events: created_by foreign key should be indexed
CREATE INDEX IF NOT EXISTS idx_events_created_by ON events(created_by);

-- Applications: reviewed_by foreign key should be indexed
CREATE INDEX IF NOT EXISTS idx_applications_reviewed_by ON applications(reviewed_by);


-- ============================================================================
-- PART 2: Array Column Indexes (GIN indexes for fast array searches)
-- ============================================================================

-- Blog posts tags for tag-based filtering
CREATE INDEX IF NOT EXISTS idx_blog_posts_tags ON blog_posts USING GIN(tags);

-- Projects tags for tag-based filtering
CREATE INDEX IF NOT EXISTS idx_projects_tags ON projects USING GIN(tags);

-- Projects tech_stack for technology filtering
CREATE INDEX IF NOT EXISTS idx_projects_tech_stack ON projects USING GIN(tech_stack);

-- Resources tags for tag-based filtering
CREATE INDEX IF NOT EXISTS idx_resources_tags ON resources USING GIN(tags);

-- Applications skills for skill-based searching/filtering
CREATE INDEX IF NOT EXISTS idx_applications_skills ON applications USING GIN(skills);


-- ============================================================================
-- PART 3: Composite Indexes for Common Query Patterns
-- ============================================================================

-- Blog posts: List published posts chronologically (most common public query)
CREATE INDEX IF NOT EXISTS idx_blog_posts_status_published_at 
ON blog_posts(status, published_at DESC) 
WHERE status = 'PUBLISHED';

-- Blog posts: Filter by category and status, ordered by date
CREATE INDEX IF NOT EXISTS idx_blog_posts_category_status_published 
ON blog_posts(category, status, published_at DESC);

-- Blog posts: Approval workflow queries (status + approval timestamp)
CREATE INDEX IF NOT EXISTS idx_blog_posts_approval_status_created 
ON blog_posts(approval_status, created_at DESC) 
WHERE approval_status = 'pending';

-- Events: List events by status and date (e.g., upcoming events)
CREATE INDEX IF NOT EXISTS idx_events_status_date 
ON events(status, date DESC);

-- Events: Type-specific event listings
CREATE INDEX IF NOT EXISTS idx_events_type_status_date 
ON events(type, status, date DESC);

-- Projects: List approved projects with featured first
CREATE INDEX IF NOT EXISTS idx_projects_status_featured_created 
ON projects(status, featured DESC, created_at DESC);

-- Projects: Approval workflow queries
CREATE INDEX IF NOT EXISTS idx_projects_status_created 
ON projects(status, created_at DESC) 
WHERE status = 'pending';

-- Resources: List approved resources by category
CREATE INDEX IF NOT EXISTS idx_resources_status_category_created 
ON resources(status, category, created_at DESC);

-- Applications: Review queue (pending applications, oldest first)
CREATE INDEX IF NOT EXISTS idx_applications_status_created 
ON applications(status, created_at ASC) 
WHERE status = 'PENDING';

-- Applications: Track reviewer workload
CREATE INDEX IF NOT EXISTS idx_applications_reviewed_by_status 
ON applications(reviewed_by, status);


-- ============================================================================
-- PART 4: Partial Indexes for Frequently Filtered Subsets
-- ============================================================================

-- Events: Active events only (exclude past events from common queries)
CREATE INDEX IF NOT EXISTS idx_events_active 
ON events(date DESC, status) 
WHERE status IN ('upcoming', 'ongoing');

-- Blog posts: Published posts only (for public listing)
CREATE INDEX IF NOT EXISTS idx_blog_posts_published 
ON blog_posts(published_at DESC, category) 
WHERE status = 'PUBLISHED';

-- Projects: Approved projects only (for public listing)
CREATE INDEX IF NOT EXISTS idx_projects_approved 
ON projects(featured DESC, created_at DESC) 
WHERE status = 'approved';

-- Projects: Featured projects (fast access to featured content)
CREATE INDEX IF NOT EXISTS idx_projects_featured_approved 
ON projects(created_at DESC) 
WHERE featured = TRUE AND status = 'approved';

-- Contacts: Unreplied contacts (for support queue)
CREATE INDEX IF NOT EXISTS idx_contacts_unreplied 
ON contacts(created_at ASC) 
WHERE replied = FALSE;


-- ============================================================================
-- PART 5: Updated_at and Created_at Indexes for Pagination/Sorting
-- ============================================================================

-- These are useful for "recently updated" queries and pagination

-- Blog posts: Recently updated (for admin/management views)
CREATE INDEX IF NOT EXISTS idx_blog_posts_updated_at ON blog_posts(updated_at DESC);

-- Projects: Recently updated
CREATE INDEX IF NOT EXISTS idx_projects_updated_at ON projects(updated_at DESC);

-- Events: Recently updated
CREATE INDEX IF NOT EXISTS idx_events_updated_at ON events(updated_at DESC);

-- Resources: Recently created (already have updated_at from trigger)
CREATE INDEX IF NOT EXISTS idx_resources_created_at ON resources(created_at DESC);


-- ============================================================================
-- PART 6: Performance Comments and Query Pattern Documentation
-- ============================================================================

COMMENT ON INDEX idx_blog_posts_status_published_at IS 
'Optimizes public blog listing: WHERE status = PUBLISHED ORDER BY published_at DESC';

COMMENT ON INDEX idx_blog_posts_category_status_published IS 
'Optimizes category filtering: WHERE category = ? AND status = PUBLISHED ORDER BY published_at DESC';

COMMENT ON INDEX idx_events_status_date IS 
'Optimizes event listing: WHERE status = ? ORDER BY date DESC';

COMMENT ON INDEX idx_projects_status_featured_created IS 
'Optimizes project listing with featured priority: WHERE status = approved ORDER BY featured DESC, created_at DESC';

COMMENT ON INDEX idx_applications_status_created IS 
'Optimizes pending applications queue: WHERE status = PENDING ORDER BY created_at ASC';

COMMENT ON INDEX idx_blog_posts_tags IS 
'Enables fast tag searches: WHERE ? = ANY(tags) or WHERE tags @> ARRAY[?]';

COMMENT ON INDEX idx_projects_tech_stack IS 
'Enables fast technology searches: WHERE ? = ANY(tech_stack) or WHERE tech_stack && ARRAY[?]';
