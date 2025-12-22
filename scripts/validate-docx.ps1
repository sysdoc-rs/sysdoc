# validate-docx.ps1 - Build test fixtures and validate DOCX output with OOXML Validator
#
# Prerequisites:
#   - .NET SDK installed
#   - OOXMLValidator installed: dotnet tool install -g OOXMLValidator
#
# Usage:
#   .\scripts\validate-docx.ps1
#   .\scripts\validate-docx.ps1 -InstallValidator

param(
    [switch]$InstallValidator
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$FixturesDir = Join-Path $ProjectRoot "tests\fixtures"
$BuildDir = Join-Path $ProjectRoot "target\docx-validation"

# Install validator if requested
if ($InstallValidator) {
    Write-Host "Installing OOXMLValidator..."
    dotnet tool install -g OOXMLValidator 2>$null
    if ($LASTEXITCODE -ne 0) {
        dotnet tool update -g OOXMLValidator
    }
    exit 0
}

# Check if OOXMLValidator is available
# $validatorPath = Get-Command OOXMLValidator -ErrorAction SilentlyContinue
$validatorPath = "OOXMLValidatorCLI.exe"

if (-not $validatorPath) {
    Write-Host "Warning: OOXMLValidator not found. Install with:" -ForegroundColor Yellow
    Write-Host "  dotnet tool install -g OOXMLValidator"
    Write-Host ""
    Write-Host "Alternatively, run: .\scripts\validate-docx.ps1 -InstallValidator"
    exit 1
}

# Build sysdoc if needed
Write-Host "Building sysdoc..."
Push-Location $ProjectRoot
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build sysdoc"
    }
}
finally {
    Pop-Location
}

$Sysdoc = Join-Path $ProjectRoot "target\release\sysdoc.exe"

# Create build directory
if (-not (Test-Path $BuildDir)) {
    New-Item -ItemType Directory -Path $BuildDir | Out-Null
}

# Track results
$Total = 0
$Passed = 0
$Failed = 0
$FailedTests = @()

# Test fixtures
$TestCases = @(
    "test-normal-text",
    "test-italics",
    "test-bold",
    "test-strikethrough",
    "test-png-image",
    "test-svg-image",
    "test-csv-table",
    "test-inline-table"
)

Write-Host ""
Write-Host "========================================="
Write-Host "DOCX Validation Test Suite"
Write-Host "========================================="
Write-Host ""

foreach ($testCase in $TestCases) {
    $Total++
    $TestDir = Join-Path $FixturesDir $testCase
    $OutputFile = Join-Path $BuildDir "$testCase.docx"

    Write-Host -NoNewline "Testing $testCase... "

    # Build the fixture
    $buildOutput = & $Sysdoc build $TestDir -o $OutputFile 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "BUILD FAILED" -ForegroundColor Red
        $Failed++
        $FailedTests += "$testCase (build failed)"
        continue
    }

    # Validate with OOXMLValidator
    $validationOutput = & $validatorPath $OutputFile 2>&1 | Out-String

    # Check if validation passed (empty JSON array [] means no errors)
    if ($validationOutput -match '^\s*\[\s*\]\s*$' -or $validationOutput -match '"errors"\s*:\s*\[\s*\]') {
        Write-Host "PASSED" -ForegroundColor Green
        $Passed++
    }
    elseif ($validationOutput -match 'error|invalid|failed') {
        Write-Host "FAILED" -ForegroundColor Red
        $Failed++
        $FailedTests += $testCase
        Write-Host "    Validation errors:"
        $validationOutput -split "`n" | Select-Object -First 20 | ForEach-Object { Write-Host "    $_" }
    }
    else {
        # No errors found in output
        Write-Host "PASSED" -ForegroundColor Green
        $Passed++
    }
}

Write-Host ""
Write-Host "========================================="
Write-Host "Results: $Passed/$Total passed"
Write-Host "========================================="

if ($Failed -gt 0) {
    Write-Host "Failed tests:" -ForegroundColor Red
    foreach ($test in $FailedTests) {
        Write-Host "  - $test"
    }
    exit 1
}
else {
    Write-Host "All tests passed!" -ForegroundColor Green
    exit 0
}
