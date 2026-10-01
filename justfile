#!/usr/bin/env just --justfile
#
# Development tasks for sysdoc. Run `just` to see the list.
#
# Requires: cargo-deny, cargo-audit (see `just install-tools`).
# The dev container installs these for you.

# Version of the OOXML validator used by `just validate-docx`.
export OOXML_VALIDATOR_VERSION := "2.1.6"

# List the available recipes.
default:
    @just --list

# Run every check: format, lint, test, release build, docs, licenses, advisories.
check: fmt-check clippy test build doc deny audit
    @echo "All checks passed."

# Build the release binary at target/release/sysdoc.
build:
    cargo build --release

# Build the debug binary at target/debug/sysdoc.
build-debug:
    cargo build

# Run sysdoc from source, e.g. `just run build examples/minimal-sdd -o out.pdf`.
run *ARGS:
    cargo run -- {{ ARGS }}

# Format all Rust sources in place.
fmt:
    cargo fmt

# Fail if any Rust source is not formatted.
fmt-check:
    cargo fmt --check

# Lint with clippy, treating every warning as an error.
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run the unit and integration tests.
test:
    cargo test

# Build the API documentation, including private items.
doc:
    cargo doc --no-deps --document-private-items

# Build the API documentation and open it in a browser.
doc-open:
    cargo doc --no-deps --document-private-items --open

# Check dependency licenses and security advisories (cargo-deny).
deny:
    cargo deny check

# Scan dependencies for known vulnerabilities (cargo-audit).
audit:
    cargo audit

# Validate the DOCX test fixtures against the OOXML schema.
[unix]
validate-docx:
    ./scripts/validate-docx.sh

[windows]
validate-docx:
    powershell -ExecutionPolicy Bypass -File scripts/validate-docx.ps1

# Download the OOXML validator needed by `just validate-docx`.
[unix]
install-ooxml-validator:
    ./scripts/validate-docx.sh --install-validator

[windows]
install-ooxml-validator:
    powershell -ExecutionPolicy Bypass -File scripts/validate-docx.ps1 -InstallValidator

# Install the cargo subcommands that `just check` depends on.
install-tools:
    cargo install cargo-binstall --locked
    cargo binstall --no-confirm cargo-deny cargo-audit

# Install sysdoc into ~/.cargo/bin.
install:
    cargo install --path sysdoc --locked

# Remove all build artifacts.
clean:
    cargo clean
