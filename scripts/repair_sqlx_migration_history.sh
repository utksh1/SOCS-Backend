#!/usr/bin/env bash
# Repairs only the known legacy SQLx history mismatch in databases that ran
# email/token/rate-limit migrations under 20260723000000..2. It never changes
# application tables; it only corrects _sqlx_migrations after taking a backup.
set -euo pipefail

if [[ "${1:-}" != "--apply" ]]; then
  echo "Usage: DATABASE_URL=... $0 --apply" >&2
  echo "This is a one-time repair for the legacy 20260723 migration aliases." >&2
  exit 2
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL must be set." >&2
  exit 2
fi

if ! command -v psql >/dev/null; then
  echo "psql is required." >&2
  exit 2
fi

if command -v sha384sum >/dev/null; then
  checksum() { sha384sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null; then
  checksum() { shasum -a 384 "$1" | awk '{print $1}'; }
else
  echo "sha384sum or shasum is required." >&2
  exit 2
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
migrations_dir="$root_dir/migrations"
backup_dir="${SQLX_MIGRATION_BACKUP_DIR:-$root_dir/migration-backups}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_file="$backup_dir/sqlx_migrations_before_repair_$timestamp.sql"

alias_count="$(psql "$DATABASE_URL" -Atv ON_ERROR_STOP=1 -c "
  SELECT COUNT(*)
  FROM _sqlx_migrations
  WHERE version IN (20260723000000, 20260723000001, 20260723000002)
")"
if [[ "$alias_count" != "3" ]]; then
  echo "Expected exactly the three legacy 20260723 alias rows; refusing to modify this database." >&2
  exit 1
fi

schema_ready="$(psql "$DATABASE_URL" -Atv ON_ERROR_STOP=1 -c "
  SELECT (
    to_regclass('public.verification_tokens') IS NOT NULL
    AND to_regclass('public.token_rate_limits') IS NOT NULL
    AND EXISTS (
      SELECT 1 FROM information_schema.columns
      WHERE table_schema = 'public' AND table_name = 'users' AND column_name = 'email_verified_at'
    )
  )::int
")"
if [[ "$schema_ready" != "1" ]]; then
  echo "Verification, token, and rate-limit schema is not present; use normal sqlx migrations instead." >&2
  exit 1
fi

mkdir -p "$backup_dir"
psql "$DATABASE_URL" -Atv ON_ERROR_STOP=1 -c "
  SELECT format(
    'INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) VALUES (%s, %L, %L, %L, decode(%L, ''hex''), %s);',
    version, description, installed_on, success, encode(checksum, 'hex'), execution_time
  )
  FROM _sqlx_migrations
  ORDER BY version
" > "$backup_file"
chmod 600 "$backup_file"

sql="BEGIN;
DELETE FROM _sqlx_migrations
WHERE version IN (20260723000000, 20260723000001, 20260723000002);
"

# Only migrations through 20260722000002 were already represented in the
# affected database. Newer migrations must remain pending and be applied by
# SQLx normally.
for migration in "$migrations_dir"/*.sql; do
  filename="$(basename "$migration")"
  version="${filename%%_*}"
  if (( version > 20260722000002 )); then
    continue
  fi

  description="${filename#*_}"
  description="${description%.sql}"
  description="${description//_/ }"
  digest="$(checksum "$migration")"
  sql+="
INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
VALUES ($version, '$description', NOW(), true, decode('$digest', 'hex'), 0)
ON CONFLICT (version) DO UPDATE
SET description = EXCLUDED.description,
    success = true,
    checksum = EXCLUDED.checksum;
"
done
sql+="COMMIT;"

printf '%s\n' "$sql" | psql "$DATABASE_URL" -v ON_ERROR_STOP=1 >/dev/null
echo "SQLx history repaired. Backup written to: $backup_file"
echo "Now run: sqlx migrate run"
