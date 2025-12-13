# sysdoc

A Rust-based CLI tool for creating and building Systems Engineering documents using Markdown, DrawIO, and CSV files.

## Overview

`sysdoc` provides tooling and templates to assist in writing Systems Engineering documents using modern development workflows. Write your documents in Markdown with Git version control, then build them into professional `.docx` files.

### Key Features

- 📝 **Markdown-based** - Write content in plain text with Markdown formatting
- 🎨 **DrawIO diagrams** - Embed `.drawio.svg` diagrams directly
- 📊 **CSV tables** - Use CSV files for easy table management
- 📁 **Folder-based structure** - Organize content in nested folders
- 🔄 **Auto heading depth** - Heading levels adjust automatically based on folder depth
- 📋 **DID templates** - Initialize from standards like DI-IPSC-81435B
- ✅ **Validation** - Check for broken links and missing files
- 🔧 **Git-friendly** - Perfect for version control and PR workflows

## Installation

### From Source

```bash
git clone https://github.com/yourusername/sysdoc.git
cd sysdoc
cargo install --path sysdoc
```

### Using Cargo (once published)

```bash
cargo install sysdoc
```

## Quick Start

```bash
# Initialize a new Software Design Description
sysdoc init DI-IPSC-81435B my-sdd --title "My Software Design"
cd my-sdd

# Edit your content
# ... edit Markdown files, add diagrams, etc.

# Build to .docx
sysdoc build -o output.docx

# Validate document structure
sysdoc validate --check-links --check-images
```

## Building from Source

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo

### Quick Build

```bash
# Windows
build.bat

# Linux/Mac
./build.sh
```

### Manual Build Steps

```bash
# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test

# Build release binary
cargo build --release

# Generate documentation
cargo doc --no-deps
```

The release binary will be at `target/release/sysdoc` (or `sysdoc.exe` on Windows).

### Development Tools

**Required:**

- `cargo-deny` - License and advisory checking

  ```bash
  cargo install cargo-deny
  cargo deny check
  ```

- `cargo-audit` - Security vulnerability scanner

  ```bash
  cargo install cargo-audit
  cargo audit
  ```

**Recommended:**

- `rust-analyzer` - IDE support
- VSCode with Rust extension

## Usage

### Initialize a New Document

```bash
# From a template
sysdoc init DI-IPSC-81435B my-doc

# List available templates
sysdoc list-templates
```

### Build a Document

```bash
# Basic build
sysdoc build -o output.docx

# With options
sysdoc build --input ./docs --output ../deliverables/sdd.docx --verbose

# Watch mode (auto-rebuild on changes)
sysdoc build --watch
```

### Validate

```bash
# Basic validation
sysdoc validate

# Comprehensive checks
sysdoc validate --check-links --check-images --check-tables
```

## Examples

See the [`examples/`](examples/) directory for complete examples:

- **minimal-sdd** - Simple SDD with basic features
- **complete-sdd** - Comprehensive example with all features
- **templates/** - Document templates for `sysdoc init`

All examples use section number prefixes (e.g., `01.01_purpose.md`) and are organized under `src/` directories.

Build an example:

```bash
cd examples/minimal-sdd/src
sysdoc build -o minimal-sdd.docx
```

## Documentation

- [Tutorial](docs/tutorial.md) - Step-by-step guide
- [Examples](examples/) - Example documents
- [Templates](examples/templates/) - Document templates

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

Contributions are welcome! Please ensure:

1. Code is formatted: `cargo fmt`
2. Clippy passes: `cargo clippy -- -D warnings`
3. Tests pass: `cargo test`
4. Security checks pass: `cargo deny check` and `cargo audit`

Or simply run the build script:

```bash
# Windows
build.bat

# Linux/Mac
./build.sh
```
