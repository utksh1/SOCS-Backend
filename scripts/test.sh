#!/bin/bash
set -e

echo "Running SOCS Backend Tests..."

# Ensure test DB script has run
./scripts/setup_test_db.sh

# Run tests
echo "Starting cargo test..."
cargo test --all-targets --all-features "$@"

echo "Tests completed successfully!"
