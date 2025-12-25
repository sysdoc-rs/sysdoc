# Test Fixtures

Minimal test documents for validating sysdoc's markdown to DOCX conversion.

## Test Cases

| Directory              | Feature Tested                       |
| ---------------------- | ------------------------------------ |
| `test-normal-text`     | Basic paragraph text                 |
| `test-italics`         | Italic text formatting (*text*)      |
| `test-bold`            | Bold text formatting (**text**)      |
| `test-strikethrough`   | Strikethrough formatting (~~text~~)  |
| `test-png-image`       | PNG image embedding                  |
| `test-svg-image`       | SVG/DrawIO image embedding           |
| `test-csv-table`       | CSV file table references            |
| `test-inline-table`    | Inline markdown tables               |
| `test-lists`           | Numbered and bulleted lists          |

## Setup

Generate the test template (required for DOCX output):

```bash
cargo run --bin generate_test_template
```

This creates `tests/fixtures/template.docx` which all test fixtures reference.

## Running Tests

### Build Individual Fixture

```bash
# DOCX output
sysdoc build tests/fixtures/test-normal-text -o output.docx

# Markdown output (no template needed)
sysdoc build tests/fixtures/test-normal-text -f markdown -o output/
```

### Run Full Validation Suite

The validation script builds all fixtures and validates them with OOXML Validator:

**Linux/macOS (Bash):**

```bash
# Install OOXML Validator (one-time setup)
./scripts/validate-docx.sh --install-validator

# Run validation
./scripts/validate-docx.sh
```

**Windows (PowerShell):**

```powershell
# Install OOXML Validator (one-time setup)
.\scripts\validate-docx.ps1 -InstallValidator

# Run validation
.\scripts\validate-docx.ps1
```

This is also run automatically in CI.

## OOXML Validator

[OOXML-Validator](https://github.com/mikeebowen/OOXML-Validator) is a cross-platform tool that validates Office Open XML files against the ECMA-376 and ISO/IEC 29500 standards.

**Installation:**

```bash
# Linux/macOS
./scripts/validate-docx.sh --install-validator

# Windows (PowerShell)
.\scripts\validate-docx.ps1 -InstallValidator
```

Or download directly from [GitHub Releases](https://github.com/mikeebowen/OOXML-Validator/releases).

**Usage:** `OOXMLValidatorCLI file.docx`

**Output:** JSON array of validation errors (empty `[]` means valid)

Works on Windows, Linux, and macOS as a self-contained binary (no .NET runtime required).

## Adding New Tests

1. Create a new directory: `test-<feature-name>/`
2. Copy `sysdoc.toml` from an existing test and update:
   - `document_id`
   - `document_title`
   - Keep `docx_template_path = "../template.docx"`
3. Add `src/01_test.md` with markdown testing the feature
4. Add any required assets (images, CSV files)
5. Add the test case name to `scripts/validate-docx.sh` and `scripts/validate-docx.ps1`
6. Add the test case to the integration test in `tests/integration_test.rs`
