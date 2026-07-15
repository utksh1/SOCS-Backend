#!/bin/bash
set -e

echo "Setting up test environment..."

# Check if .env.test exists, if not create it
if [ ! -f .env.test ]; then
    echo "Creating .env.test..."
    cat > .env.test << EOL
DATABASE_URL=postgres://postgres:postgres@localhost:5432/socs_test
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=test_secret_key_12345
JWT_EXPIRES_IN=1h
REFRESH_TOKEN_EXPIRES_IN=7d
CORS_ORIGIN=http://localhost:3000
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=test
SMTP_PASSWORD=test
SMTP_FROM_EMAIL=test@example.com
EOL
    echo "Created .env.test"
else
    echo ".env.test already exists"
fi

# Try to create the base test database
# (sqlx::test will create random DBs from this base, but we need the base to exist or have permissions to create DBs)
echo "Ensuring Postgres is available on localhost:5432..."
if command -v psql &> /dev/null; then
    psql -h localhost -U postgres -c "CREATE DATABASE socs_test;" 2>/dev/null || true
    echo "Base test database ready."
else
    echo "psql not found. Please ensure your local Postgres is running and you have created the 'socs_test' database if it doesn't exist."
fi

echo "Test environment setup complete!"
