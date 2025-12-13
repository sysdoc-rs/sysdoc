# sysdoc Documentation

Welcome to the sysdoc documentation!

## Getting Started

- [Tutorial](tutorial.md) - Step-by-step guide to using sysdoc

## Resources

- [Examples](../examples/) - Example documents demonstrating sysdoc features
- [Templates](../examples/templates/) - Document templates for `sysdoc init`

## Quick Links

### For Users

- **First time?** Start with the [Tutorial](tutorial.md)
- **Looking for examples?** Check the [examples directory](../examples/)
- **Need a template?** Use `sysdoc list-templates` or browse [templates](../examples/templates/)

### For Developers

- **Contributing?** See the main [README](../README.md)
- **Running tests?** Use `cargo test`
- **Building?** Use `cargo build --release`

## Document Structure

sysdoc documents follow a simple convention:

```
my-document/
├── 01-section/
│   ├── index.md           # Section content
│   ├── diagrams/
│   │   └── *.drawio.svg   # DrawIO diagrams
│   ├── tables/
│   │   └── *.csv          # CSV tables
│   └── 01-subsection/
│       └── index.md
└── 02-next-section/
    └── index.md
```

## Features

- ✅ Markdown-based content
- ✅ DrawIO SVG diagrams
- ✅ PNG images
- ✅ CSV tables
- ✅ Auto-nested heading depth
- ✅ Git-friendly workflow
- ✅ Template system
- ✅ Document validation

## Command Reference

```bash
# Initialize a new document
sysdoc init <template> [path] [options]

# Build a document
sysdoc build [options]

# Validate document structure
sysdoc validate [options]

# List available templates
sysdoc list-templates
```

For detailed command usage, run `sysdoc <command> --help`.
