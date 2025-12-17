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

Early development stage - the project is being scaffolded and initial architecture decisions are being made.
