# validate-docx.ps1 - Build test fixtures and validate DOCX output with OOXML-Validator
#
# Prerequisites:
#   - OOXML-Validator binary (from https://github.com/mikeebowen/OOXML-Validator)
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
$ValidatorVersion = if ($env:OOXML_VALIDATOR_VERSION) { $env:OOXML_VALIDATOR_VERSION } else { "2.1.6" }
$ValidatorInstallDir = Join-Path $env:LOCALAPPDATA "ooxml-validator"

# Install validator if requested
if ($InstallValidator) {
    Write-Host "Installing OOXML-Validator v$ValidatorVersion for win-x64..."

    if (-not (Test-Path $ValidatorInstallDir)) {
        New-Item -ItemType Directory -Path $ValidatorInstallDir | Out-Null
    }

    $downloadUrl = "https://github.com/mikeebowen/OOXML-Validator/releases/download/v$ValidatorVersion/win-x64.zip"
    $zipPath = Join-Path $env:TEMP "ooxml-validator.zip"

    Write-Host "Downloading from: $downloadUrl"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath

    Write-Host "Extracting to: $ValidatorInstallDir"
    Expand-Archive -Path $zipPath -DestinationPath $ValidatorInstallDir -Force
    Remove-Item $zipPath

    Write-Host ""
    Write-Host "Installed successfully!" -ForegroundColor Green
    Write-Host "Add to PATH: `$env:PATH += `";$ValidatorInstallDir`""
    exit 0
}

# Find OOXML-Validator binary
function Find-Validator {
    # Check PATH first
    $pathValidator = Get-Command "OOXMLValidatorCLI.exe" -ErrorAction SilentlyContinue
    if ($pathValidator) {
        return $pathValidator.Source
    }

    # Check common install locations
    $locations = @(
        (Join-Path $ValidatorInstallDir "OOXMLValidatorCLI.exe"),
        (Join-Path $env:LOCALAPPDATA "ooxml-validator\OOXMLValidatorCLI.exe"),
        ".\OOXMLValidatorCLI.exe"
    )

    foreach ($loc in $locations) {
        if (Test-Path $loc) {
            return $loc
        }
    }

    return $null
}

$validatorPath = Find-Validator

if (-not $validatorPath) {
    Write-Host "Warning: OOXML-Validator not found." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Install with:"
    Write-Host "  .\scripts\validate-docx.ps1 -InstallValidator"
    Write-Host ""
    Write-Host "Or download manually from:"
    Write-Host "  https://github.com/mikeebowen/OOXML-Validator/releases"
    exit 1
}

Write-Host "Using validator: $validatorPath"

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
Write-Host "Using: OOXML-Validator v$ValidatorVersion"
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

    # Get absolute path for validator
    $absOutputFile = (Resolve-Path $OutputFile).Path

    # Validate with OOXML-Validator (outputs JSON by default)
    $validationOutput = & $validatorPath $absOutputFile 2>&1 | Out-String

    # OOXML-Validator returns empty array [] for valid documents
    # or array of error objects for invalid documents
    if ($validationOutput -match '^\s*\[\s*\]\s*$') {
        Write-Host "PASSED" -ForegroundColor Green
        $Passed++
    }
    elseif ($validationOutput -match '"Description"') {
        # JSON array with error objects
        Write-Host "FAILED" -ForegroundColor Red
        $Failed++
        $FailedTests += $testCase
        Write-Host "    Validation errors:"
        $validationOutput -split "`n" | Select-Object -First 20 | ForEach-Object { Write-Host "    $_" }
    }
    else {
        # Check if output is empty or just whitespace (valid)
        $trimmed = $validationOutput.Trim()
        if ([string]::IsNullOrEmpty($trimmed) -or $trimmed -eq "[]") {
            Write-Host "PASSED" -ForegroundColor Green
            $Passed++
        }
        else {
            # Unknown output format, treat as error
            Write-Host "UNKNOWN" -ForegroundColor Yellow
            Write-Host "    Output: $validationOutput"
            $Failed++
            $FailedTests += "$testCase (unknown validator output)"
        }
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
