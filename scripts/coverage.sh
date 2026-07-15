#!/bin/bash
set -e

echo "Generating code coverage report using cargo-tarpaulin..."

if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "cargo-tarpaulin is not installed. Installing it now..."
    cargo install cargo-tarpaulin
fi

# Run tarpaulin
cargo tarpaulin --ignore-tests -o html -o xml --output-dir coverage/

echo "Coverage report generated at coverage/tarpaulin-report.html"
