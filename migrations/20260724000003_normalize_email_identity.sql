-- Treat email addresses as canonical identifiers. Without this, casing can
-- create duplicate accounts, registrations, or independent reset-limit
-- buckets even though users perceive the addresses as the same identity.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM users
        GROUP BY LOWER(BTRIM(email))
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'Cannot normalize users.email: duplicate email addresses differ only by case or whitespace. Resolve those accounts before retrying this migration.';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM event_registrations
        GROUP BY event_id, LOWER(BTRIM(email))
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'Cannot normalize event registration emails: duplicate registrations differ only by case or whitespace. Resolve them before retrying this migration.';
    END IF;
END $$;

UPDATE users
SET email = LOWER(BTRIM(email))
WHERE email IS DISTINCT FROM LOWER(BTRIM(email));

UPDATE event_registrations
SET email = LOWER(BTRIM(email))
WHERE email IS DISTINCT FROM LOWER(BTRIM(email));

UPDATE applications
SET email = LOWER(BTRIM(email))
WHERE email IS DISTINCT FROM LOWER(BTRIM(email));

UPDATE contacts
SET email = LOWER(BTRIM(email))
WHERE email IS DISTINCT FROM LOWER(BTRIM(email));

-- Merge legacy rate-limit rows before adding the normalized uniqueness guard.
CREATE TEMPORARY TABLE normalized_token_rate_limits ON COMMIT DROP AS
SELECT
    LOWER(BTRIM(email)) AS email,
    token_type,
    LEAST(SUM(attempt_count), 2147483647)::INTEGER AS attempt_count,
    window_start,
    MIN(created_at) AS created_at
FROM token_rate_limits
GROUP BY LOWER(BTRIM(email)), token_type, window_start;

TRUNCATE token_rate_limits;

INSERT INTO token_rate_limits (email, token_type, attempt_count, window_start, created_at)
SELECT email, token_type, attempt_count, window_start, created_at
FROM normalized_token_rate_limits;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_normalized
    ON users (LOWER(BTRIM(email)));

CREATE UNIQUE INDEX IF NOT EXISTS idx_event_registrations_email_normalized
    ON event_registrations (event_id, LOWER(BTRIM(email)));

CREATE UNIQUE INDEX IF NOT EXISTS idx_rate_limits_normalized_unique
    ON token_rate_limits (LOWER(BTRIM(email)), token_type, window_start);
