# sysdoc

## Project Overview

`sysdoc` is a Rust-based system documentation tool currently in early development.
`sysdoc` attempts to provide tooling and templates to assist in writing Systems Engineering documents using Markdown, DrawIO, and CSV organized into folders for each section stored in a single Git repository for each document.
The intention is to support a workflows using VS Code, Git, and Pull Request review process to create Systems Engineering Documents faster and with less friction than working in Word or Visio.

### Key features

- Initialize templates based on DID standards such as `DI-IPSC-81435B - Software Design Description (SDD)`
- Aggregate multiple Markdown files in a nested folder structure with auto nested heading depth
- Support `.drawio.svg` and `.png` image files
- Support CSV files for tables
- Generate `.docx` files containing the Markdown, image, and table content
- Generate an aggregated `.md` file and `images/` folder

### Future features

- ID markers in Markdown comments to support automatic traceability table generation
- GUI for interactively exploring and searching multiple systems engineering document repositories using traceability and Git history

## Project Structure

This is a Cargo workspace with the following structure:
- `/sysdoc/` - Main crate containing the sysdoc application
- `Cargo.toml` - Workspace configuration at the root

## Technology Stack

- **Language**: Rust (Edition 2021)
- **Build System**: Cargo workspace

## Development Guidelines

### Code Style

- Follow standard Rust conventions and idioms
- Use `rustfmt` for code formatting
- Use `clippy` for linting
- Prefer descriptive variable and function names
- Add documentation comments (`///`) for public APIs

### Testing

- Write unit tests in the same file as the code being tested
- Use integration tests in the `tests/` directory for end-to-end functionality
- Run tests with `cargo test`
- Aim for meaningful test coverage of core functionality

### Building

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the application
cargo run

# Run tests
cargo test

# Run clippy
cargo clippy

# Format code
cargo fmt

# Check licenses and security advisories
cargo deny check
cargo audit
```

### Development Tools

**Required security tools:**

- `cargo-deny` - License and advisory checker

  ```bash
  cargo install cargo-deny
  ```

  Enforces permissive open source licenses and checks for security vulnerabilities. Configuration in `deny.toml` allows only permissive licenses (MIT, Apache-2.0, BSD, etc.) and denies copyleft licenses (GPL, LGPL, AGPL).

- `cargo-audit` - Security vulnerability scanner

  ```bash
  cargo install cargo-audit
  ```

  Scans dependencies for known security vulnerabilities from the RustSec Advisory Database.

## Architecture

[To be defined as the project develops]

## Dependencies

Currently, the project has minimal dependencies. New dependencies should be:
- Well-maintained and widely used in the Rust ecosystem
- Added to `[workspace.dependencies]` in the root `Cargo.toml` for shared dependencies
- Justified with a clear use case

## Current Status

Preparing for v1.0.0 release on crates.io.

## v1.0.0 Roadmap

### 1. Implement Validation Command

- [x] Implement `sysdoc validate` with all checks:
  - Broken image references (always validated)
  - Broken CSV references (always validated)
  - Broken internal markdown links (always validated, --check-links removed)
  - sysdoc.toml configuration validity
- [x] Exit with non-zero code when validation fails
- **Files**: `sysdoc/src/main.rs`, `sysdoc/src/source_model.rs`, `sysdoc/src/source_model/validation.rs`

### 2. Integrate Validation into Build

- [x] Run validation automatically before building
- [x] Fail the build if validation fails
- **Files**: `sysdoc/src/main.rs`, `sysdoc/src/pipeline.rs`

### 3. Remove Unimplemented --watch Flag

- [ ] Remove `--watch` / `-w` flag from build command
- [ ] Remove any related code paths
- **Files**: `sysdoc/src/cli.rs`, `sysdoc/src/main.rs`

### 4. Add Crates.io Metadata

- [ ] Add to `sysdoc/Cargo.toml`:
  - `description` - Short description of the crate
  - `repository` - GitHub repository URL
  - `homepage` - Project homepage (can be same as repository)
  - `keywords` - Up to 5 keywords for discovery
  - `categories` - "command-line-utilities", "text-processing"
- [ ] Update `authors` in workspace `Cargo.toml`
- **Files**: `Cargo.toml`, `sysdoc/Cargo.toml`

### 5. Add Crates.io Publish Workflow

- [ ] Add `publish` job to GitHub Actions CI
- [ ] Trigger on version tags (v*)
- [ ] Use `cargo publish` with CARGO_REGISTRY_TOKEN secret
- **Files**: `.github/workflows/ci.yml`

### 6. Update README.md

- [ ] Add badges: crates.io version, CI status, license
- [ ] Fix placeholder GitHub URL (yourusername)
- [ ] Remove `--watch` flag references
- [ ] Update output formats section (add HTML, PDF)
- **Files**: `README.md`

### 7. Update CLAUDE.md

- [ ] Add Architecture section describing three-stage pipeline
- [ ] Update "Current Status" from early development to v1.0.0
- [ ] Add this task list
- **Files**: `CLAUDE.md`

### 8. Bump Version to 1.0.0

- [ ] Update version in workspace `Cargo.toml`
- [ ] Update version in `sysdoc/Cargo.toml`
- [ ] Create release tag after all tasks complete
- **Files**: `Cargo.toml`, `sysdoc/Cargo.toml`

### Implementation Order

1. Validation command (largest task)
2. Integrate validation into build
3. Remove --watch flag
4. Crates.io metadata
5. README updates
6. CLAUDE.md updates
7. CI publish workflow
8. Version bump and tag

### Verification

After implementation:

1. `cargo test` - all tests pass
2. `cargo clippy -- -D warnings` - no warnings
3. `cargo fmt --check` - properly formatted
4. `cargo deny check` - license compliance
5. `sysdoc validate examples/minimal-sdd` - validation works
6. `sysdoc build examples/minimal-sdd -o test.docx` - build runs validation
7. `cargo publish --dry-run` - ready for crates.io
