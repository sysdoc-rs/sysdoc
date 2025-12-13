# sysdoc Examples

This directory contains example sysdoc projects demonstrating various features and use cases.

## Examples

### minimal-sdd
A bare minimum Software Design Description (SDD) demonstrating basic structure and features.
- Simple folder structure
- Basic Markdown content
- Single diagram
- Demonstrates core functionality

**Build it:**
```bash
cd minimal-sdd
sysdoc build -o minimal-sdd.docx
```

### complete-sdd
A comprehensive Software Design Description showcasing all sysdoc features.
- Complex nested folder structure
- DrawIO SVG diagrams
- CSV tables
- Multiple sections
- Auto-nested heading depth

**Build it:**
```bash
cd complete-sdd
sysdoc build -o complete-sdd.docx
```

## Templates

The `templates/` directory contains raw templates used by `sysdoc init` command.

**Initialize a new document:**
```bash
sysdoc init DI-IPSC-81435B my-new-sdd
# or
sysdoc init SDD my-new-sdd
```

## Testing

These examples are used by the integration test suite. To run tests:
```bash
cargo test
```
