#!/bin/bash
set -e

echo "Starting local test run..."

# Prepare the DB offline query cache
echo "Updating sqlx prepare cache..."
cargo sqlx prepare --workspace

echo "Running unit tests..."
cargo test --lib --bins --tests --benches

echo "Running integration tests..."
export SQLX_OFFLINE=false
cargo test --test integration_tests

echo "All tests passed successfully!"
