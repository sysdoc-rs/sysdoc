#!/bin/bash
set -e

echo "Running cargo fmt check..."
cargo fmt --check || { echo "ERROR: cargo fmt check failed"; exit 1; }

echo "Running cargo clippy..."
cargo clippy -- -D warnings || { echo "ERROR: cargo clippy failed"; exit 1; }

echo "Running cargo test..."
cargo test || { echo "ERROR: cargo test failed"; exit 1; }

echo "Running cargo build --release..."
cargo build --release || { echo "ERROR: cargo build --release failed"; exit 1; }

echo "Running cargo doc..."
cargo doc --no-deps || { echo "ERROR: cargo doc failed"; exit 1; }

echo ""
echo "========================================"
echo "Build completed successfully!"
echo "========================================"
exit 0
