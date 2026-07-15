-- The application treats MEMBER as the baseline role for every account.
-- Keep that RBAC invariant in the database as well so direct SQL and future
-- code paths cannot create an empty or privilege-only role array.

UPDATE users
SET roles = array_append(roles, 'MEMBER'::user_role)
WHERE NOT ('MEMBER'::user_role = ANY(roles));

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_roles_include_member;

ALTER TABLE users
    ADD CONSTRAINT users_roles_include_member
    CHECK (
        cardinality(roles) > 0
        AND 'MEMBER'::user_role = ANY(roles)
    );

-- Tokens are high-entropy (256-bit) and stored as deterministic SHA-256
-- fingerprints for indexed lookup; the original migration predates that fix.
COMMENT ON COLUMN verification_tokens.token_hash
    IS 'SHA-256 fingerprint of a 256-bit verification or password-reset token (never plaintext)';
