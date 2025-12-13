# sysdoc Examples

This directory contains example sysdoc projects demonstrating various features and use cases.

All document source files are organized under the `src/` subdirectory with section number prefixes for clarity.

## Examples

### minimal-sdd

A bare minimum Software Design Description (SDD) demonstrating basic structure and features.

- Simple folder structure with numbered sections
- Basic Markdown content with section prefixes (e.g., `01.01_purpose.md`)
- Single diagram
- Demonstrates core functionality

**Build it:**

```bash
cd minimal-sdd/src
sysdoc build -o minimal-sdd.docx
```

### complete-sdd

A comprehensive Software Design Description showcasing all sysdoc features.

- Complex nested folder structure with section numbering
- DrawIO SVG diagrams
- CSV tables
- Multiple sections with descriptive filenames
- Auto-nested heading depth

**Build it:**

```bash
cd complete-sdd/src
sysdoc build -o complete-sdd.docx
```

## Templates

The `templates/src/` directory contains raw templates used by `sysdoc init` command.

Templates follow a section numbering convention (e.g., `01.01_identification.md`) for easy organization.

**Initialize a new document:**

```bash
sysdoc init DI-IPSC-81435B my-new-sdd
# or
sysdoc init SDD my-new-sdd
```

## File Naming Convention

All markdown files use section number prefixes for better organization and IDE tab clarity:

- Format: `XX.YY_descriptive-name.md`
- Example: `01.01_purpose.md`, `02.03_component-summary.md`
- Section folders also use numbered prefixes: `01-introduction/`, `02-architecture/`

## Testing

These examples are used by the integration test suite. To run tests:

```bash
cargo test
```
