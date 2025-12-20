@echo off
setlocal enabledelayedexpansion

echo Running cargo fmt check...
cargo fmt --check
if !errorlevel! neq 0 (
    echo ERROR: cargo fmt check failed
    exit /b 1
)

echo Running cargo clippy...
cargo clippy -- -D warnings
if !errorlevel! neq 0 (
    echo ERROR: cargo clippy failed
    exit /b 1
)

echo Running cargo test...
cargo test
if !errorlevel! neq 0 (
    echo ERROR: cargo test failed
    exit /b 1
)

echo Running cargo build --release...
cargo build --release
if !errorlevel! neq 0 (
    echo ERROR: cargo build --release failed
    exit /b 1
)

echo Running cargo doc...
cargo doc --no-deps --document-private-items
if !errorlevel! neq 0 (
    echo ERROR: cargo doc failed
    exit /b 1
)

echo Running cargo deny check...
cargo deny check
if !errorlevel! neq 0 (
    echo ERROR: cargo deny check failed
    exit /b 1
)

echo Running cargo audit...
cargo audit
if !errorlevel! neq 0 (
    echo ERROR: cargo audit failed
    exit /b 1
)

echo.
echo ========================================
echo Build completed successfully!
echo ========================================
exit /b 0
