#!/bin/bash
# validate-docx.sh - Build test fixtures and validate DOCX output with OOXML-Validator
#
# Prerequisites:
#   - OOXML-Validator binary in PATH (from https://github.com/mikeebowen/OOXML-Validator)
#
# Usage:
#   ./scripts/validate-docx.sh [--install-validator]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FIXTURES_DIR="$PROJECT_ROOT/tests/fixtures"
BUILD_DIR="$PROJECT_ROOT/target/docx-validation"
OOXML_VALIDATOR_VERSION="${OOXML_VALIDATOR_VERSION:-2.1.6}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)
            if [ "$(uname -m)" = "aarch64" ]; then
                echo "linux-arm64"
            else
                echo "linux-x64"
            fi
            ;;
        Darwin*)
            if [ "$(uname -m)" = "arm64" ]; then
                echo "osx-arm64"
            else
                echo "osx-x64"
            fi
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "win-x64"
            ;;
        *)
            echo "linux-x64"
            ;;
    esac
}

# Install validator if requested
if [[ "$1" == "--install-validator" ]]; then
    PLATFORM=$(detect_platform)
    INSTALL_DIR="$HOME/.local/bin/ooxml-validator"

    echo "Installing OOXML-Validator v${OOXML_VALIDATOR_VERSION} for ${PLATFORM}..."
    mkdir -p "$INSTALL_DIR"

    DOWNLOAD_URL="https://github.com/mikeebowen/OOXML-Validator/releases/download/v${OOXML_VALIDATOR_VERSION}/${PLATFORM}.zip"
    echo "Downloading from: $DOWNLOAD_URL"

    curl -sL "$DOWNLOAD_URL" -o /tmp/ooxml-validator.zip
    unzip -o /tmp/ooxml-validator.zip -d "$INSTALL_DIR"

    if [[ "$PLATFORM" != "win-x64" ]]; then
        chmod +x "$INSTALL_DIR/OOXMLValidatorCLI"
    fi

    echo ""
    echo -e "${GREEN}Installed successfully!${NC}"
    echo "Add to PATH: export PATH=\"\$PATH:$INSTALL_DIR\""
    exit 0
fi

# Find OOXML-Validator binary
find_validator() {
    # Check PATH first
    if command -v OOXMLValidatorCLI &> /dev/null; then
        echo "OOXMLValidatorCLI"
        return 0
    fi

    # Check common install locations
    local locations=(
        "$HOME/.local/bin/ooxml-validator/OOXMLValidatorCLI"
        "/usr/local/bin/OOXMLValidatorCLI"
        "./OOXMLValidatorCLI"
    )

    for loc in "${locations[@]}"; do
        if [[ -x "$loc" ]]; then
            echo "$loc"
            return 0
        fi
    done

    return 1
}

VALIDATOR=$(find_validator) || {
    echo -e "${YELLOW}Warning: OOXML-Validator not found.${NC}"
    echo ""
    echo "Install with:"
    echo "  ./scripts/validate-docx.sh --install-validator"
    echo ""
    echo "Or download manually from:"
    echo "  https://github.com/mikeebowen/OOXML-Validator/releases"
    exit 1
}

echo "Using validator: $VALIDATOR"

# Build sysdoc if needed
echo "Building sysdoc..."
cd "$PROJECT_ROOT"
cargo build --release

SYSDOC="$PROJECT_ROOT/target/release/sysdoc"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" || "$OSTYPE" == "cygwin" ]]; then
    SYSDOC="$SYSDOC.exe"
fi

# Create build directory
mkdir -p "$BUILD_DIR"

# Track results
TOTAL=0
PASSED=0
FAILED=0
FAILED_TESTS=""

# Test fixtures
TEST_CASES=(
    "test-normal-text"
    "test-italics"
    "test-bold"
    "test-strikethrough"
    "test-png-image"
    "test-svg-image"
    "test-csv-table"
    "test-inline-table"
    "test-lists"
)

echo ""
echo "========================================="
echo "DOCX Validation Test Suite"
echo "Using: OOXML-Validator v${OOXML_VALIDATOR_VERSION}"
echo "========================================="
echo ""

for test_case in "${TEST_CASES[@]}"; do
    TOTAL=$((TOTAL + 1))
    TEST_DIR="$FIXTURES_DIR/$test_case"
    OUTPUT_FILE="$BUILD_DIR/${test_case}.docx"

    echo -n "Testing $test_case... "

    # Build the fixture
    if ! "$SYSDOC" build "$TEST_DIR" -o "$OUTPUT_FILE" 2>/dev/null; then
        echo -e "${RED}BUILD FAILED${NC}"
        FAILED=$((FAILED + 1))
        FAILED_TESTS="$FAILED_TESTS\n  - $test_case (build failed)"
        continue
    fi

    # Get absolute path for validator
    ABS_OUTPUT_FILE="$(cd "$(dirname "$OUTPUT_FILE")" && pwd)/$(basename "$OUTPUT_FILE")"

    # Validate with OOXML-Validator (outputs JSON by default)
    VALIDATION_OUTPUT=$("$VALIDATOR" "$ABS_OUTPUT_FILE" 2>&1) || true

    # OOXML-Validator returns empty array [] for valid documents
    # or array of error objects for invalid documents
    # Normalize the output by removing all whitespace for comparison
    TRIMMED=$(echo "$VALIDATION_OUTPUT" | tr -d '[:space:]')

    if [[ -z "$TRIMMED" || "$TRIMMED" == "[]" ]]; then
        # Empty output or empty array means valid document
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
    elif echo "$VALIDATION_OUTPUT" | grep -q '"Description"'; then
        # JSON with Description field indicates validation errors
        echo -e "${RED}FAILED${NC}"
        FAILED=$((FAILED + 1))
        FAILED_TESTS="$FAILED_TESTS\n  - $test_case"
        echo "    Validation errors:"
        echo "$VALIDATION_OUTPUT" | head -20 | sed 's/^/    /'
    else
        # Unknown output format, treat as error
        echo -e "${YELLOW}UNKNOWN${NC}"
        echo "    Output: $VALIDATION_OUTPUT"
        FAILED=$((FAILED + 1))
        FAILED_TESTS="$FAILED_TESTS\n  - $test_case (unknown validator output)"
    fi
done

echo ""
echo "========================================="
echo "Results: $PASSED/$TOTAL passed"
echo "========================================="

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}Failed tests:${NC}"
    echo -e "$FAILED_TESTS"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
