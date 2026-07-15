#!/bin/bash
set -e

echo "Running SOCS Backend Test Coverage..."

# Check if tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "cargo-tarpaulin not found. Installing..."
    cargo install cargo-tarpaulin
fi

# Ensure test DB script has run
./scripts/setup_test_db.sh

# Run coverage
echo "Starting cargo tarpaulin..."
cargo tarpaulin --ignore-tests --out Html --out Xml

echo "Coverage completed! Report available at tarpaulin-report.html"
