#!/usr/bin/env bash
set -e

DB_CONTAINER_NAME="socs-migration-test-db"
DB_USER="postgres"
DB_PASSWORD="password"
DB_NAME="socs_test"
DB_PORT="5433"

echo "Starting temporary PostgreSQL container..."
docker run --name $DB_CONTAINER_NAME \
  -e POSTGRES_USER=$DB_USER \
  -e POSTGRES_PASSWORD=$DB_PASSWORD \
  -e POSTGRES_DB=$DB_NAME \
  -p $DB_PORT:5432 \
  -d postgres:15-alpine > /dev/null

export DATABASE_URL="postgres://$DB_USER:$DB_PASSWORD@localhost:$DB_PORT/$DB_NAME"

echo "Waiting for PostgreSQL to be ready..."
until docker exec $DB_CONTAINER_NAME pg_isready -U $DB_USER > /dev/null 2>&1; do
  sleep 1
done

echo "Running migrations..."
if cargo sqlx migrate run; then
  echo "Migrations applied successfully."
  EXIT_CODE=0
else
  echo "Migrations failed."
  EXIT_CODE=1
fi

echo "Cleaning up..."
docker stop $DB_CONTAINER_NAME > /dev/null
docker rm $DB_CONTAINER_NAME > /dev/null

exit $EXIT_CODE
