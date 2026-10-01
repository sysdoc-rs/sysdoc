# sysdoc

A Rust-based CLI tool for creating and building Systems Engineering documents using Markdown, DrawIO, and CSV files.

## Overview

`sysdoc` provides tooling and templates to assist in writing Systems Engineering documents using modern development workflows. Write your documents in Markdown with Git version control, then build them into professional `.docx` or consolidated Markdown files.

### Key Features

- 📝 **Markdown-based** - Write content in plain text with Markdown formatting
- 📄 **Multiple output formats** - Build to DOCX or Markdown with images folder
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

# Build to .docx (default)
sysdoc build -o output.docx

# Or build to consolidated markdown
sysdoc build -f markdown -o output

# Validate document structure
sysdoc validate --check-links --check-images
```

## Building from Source

### Prerequisites

- Rust 1.70+ (2021 edition) and Cargo
- [`just`](https://just.systems) - command runner for the development tasks

  ```bash
  cargo install just
  ```

Then install the cargo subcommands the checks rely on (`cargo-deny` and
`cargo-audit`):

```bash
just install-tools
```

### Dev Container

The repository ships a [dev container](.devcontainer/devcontainer.json) with the
Rust toolchain, `just`, `cargo-deny` and `cargo-audit` preinstalled. Open the
folder in VS Code and choose **Reopen in Container**, or use the GitHub Codespaces
button, and everything below works without further setup.

### Build

```bash
# Build the release binary (target/release/sysdoc)
just build

# Run every check: format, lint, test, release build, docs, licenses, advisories
just check

# List all available tasks
just
```

The release binary will be at `target/release/sysdoc` (or `sysdoc.exe` on Windows).

### Individual Tasks

Each step of `just check` is also available on its own:

| Command | Description |
| --- | --- |
| `just fmt` | Format all Rust sources in place |
| `just fmt-check` | Fail if any source is unformatted |
| `just clippy` | Lint with clippy, warnings as errors |
| `just test` | Run the unit and integration tests |
| `just build` | Build the release binary |
| `just build-debug` | Build the debug binary |
| `just doc` | Build the API documentation |
| `just deny` | Check dependency licenses and advisories |
| `just audit` | Scan dependencies for known vulnerabilities |
| `just validate-docx` | Validate DOCX fixtures against the OOXML schema |
| `just run <args>` | Run sysdoc from source, e.g. `just run build examples/minimal-sdd -o out.pdf` |
| `just clean` | Remove all build artifacts |

`just validate-docx` needs the OOXML validator, which `just install-ooxml-validator`
downloads for you.

### Development Tools

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
# Basic build to DOCX (current directory)
sysdoc build -o output.docx

# Build from specific directory
sysdoc build ./docs -o output.docx

# Build to Markdown with images folder
sysdoc build -f markdown -o output-dir

# Build with format and options
sysdoc build ./docs --format docx --output ../deliverables/sdd.docx --verbose

# Watch mode (auto-rebuild on changes)
sysdoc build --watch -o my-doc.docx
```

**Output Formats:**

- **DOCX** (default) - Single Microsoft Word `.docx` file with embedded images
- **Markdown** - Consolidated markdown file with images in a separate folder

### Validate

```bash
# Basic validation (current directory)
sysdoc validate

# Validate specific directory
sysdoc validate ./docs

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

Contributions are welcome! Please run the full check suite before opening a pull
request:

```bash
just check
```

This covers formatting, clippy, tests, the release build, documentation, license
compliance and security advisories. See
[Individual Tasks](#individual-tasks) to run any of those steps on its own.
