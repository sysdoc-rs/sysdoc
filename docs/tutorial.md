# sysdoc Tutorial

This tutorial will guide you through using sysdoc to create and build systems engineering documents.

## Prerequisites

- Rust and Cargo installed
- sysdoc installed (`cargo install sysdoc` or built from source)
- A text editor (VS Code recommended)
- Git (for version control)

## Quick Start

### 1. Initialize a New Document

Create a new Software Design Description using a template:

```bash
sysdoc init DI-IPSC-81435B my-sdd --title "Flight Control Software Design"
cd my-sdd
```

This creates a directory structure based on the DI-IPSC-81435B standard.

### 2. Explore the Structure

```bash
my-sdd/
├── 01-scope/
│   └── scope.md
├── 02-referenced-documents/
│   └── referenced-documents.md
├── 03-software-design/
│   ├── software-design.md
│   ├── 01-system-wide-design/
│   │   └── system-wide-design.md
│   ├── 02-architectural-design/
│   │   └── architectural-design.md
│   └── 03-detailed-design/
│       └── detailed-design.md
├── 04-requirements-traceability/
│   └── requirements-traceability.md
└── 05-notes/
    └── notes.md
```

**Note:** Files use descriptive names (not `index.md`) so they're easily identifiable in your IDE tabs.

### 3. Edit Content

Edit the Markdown files to add your content:

```bash
code 01-scope/scope.md
```

Add content following Markdown syntax:

```markdown
# Scope

## Identification

This Software Design Description describes the design for the Flight Control System.

**Document Number:** FCS-SDD-001
**Version:** 1.0
**Date:** 2024-01-15
```

### 4. Add Diagrams

Create diagrams using DrawIO:

1. Create a `.drawio.svg` file in the appropriate section
2. Design your diagram in DrawIO
3. Save as SVG with embedded diagram data
4. Reference in Markdown:

```markdown
![System Architecture](diagrams/architecture.drawio.svg)
```

### 5. Add Tables

Create CSV files for tables:

**components.csv:**
```csv
Component,Description,Status
Flight Controller,Main control logic,Implemented
Sensor Interface,Sensor data processing,In Progress
Actuator Driver,Actuator control,Planned
```

Reference in Markdown:

```markdown
<!-- TABLE: tables/components.csv -->
```

### 6. Build the Document

Generate a .docx file:

```bash
sysdoc build -o flight-control-sdd.docx
```

With options:

```bash
sysdoc build \
  --input . \
  --output ../deliverables/sdd-v1.0.docx \
  --verbose
```

### 7. Validate the Document

Check for broken links and missing files:

```bash
sysdoc validate --check-links --check-images --check-tables
```

## Advanced Usage

### Watch Mode

Automatically rebuild when files change:

```bash
sysdoc build --watch --verbose
```

### Custom Structure

You're not limited to templates. Create your own structure:

```bash
mkdir my-custom-doc
cd my-custom-doc
mkdir -p 01-intro 02-content/subsection-a
echo "# Introduction" > 01-intro/introduction.md
sysdoc build
```

**Tip:** Use descriptive filenames instead of `index.md` for better clarity when working with multiple files in your editor.

### Nested Sections

sysdoc automatically adjusts heading levels based on folder depth:

```
01-section/
  section.md            # H1: Section
  01-subsection/
    subsection.md       # H2: Subsection
    01-sub-subsection/
      sub-subsection.md # H3: Sub-subsection
```

### Version Control

Initialize a Git repository for your document:

```bash
git init
git add .
git commit -m "Initial document structure"
```

Use branches for different versions or review workflows:

```bash
git checkout -b feature/add-security-section
# Make changes
git commit -am "Add security architecture section"
# Create PR for review
```

## Examples

See the `examples/` directory for complete examples:

- **minimal-sdd** - Simple example with basic features
- **complete-sdd** - Comprehensive example with all features

Build an example:

```bash
cd examples/minimal-sdd
sysdoc build -o output.docx
```

## Tips and Best Practices

1. **Use numbered folders** for ordered sections (01-, 02-, etc.)
2. **Keep images close to content** in the same section folder
3. **Use descriptive filenames** for diagrams and tables
4. **Commit often** to Git for version control
5. **Use PR reviews** for collaborative document development
6. **Validate regularly** to catch broken references early
7. **Keep diagrams simple** - complex diagrams don't render well in Word
8. **Use CSV for tables** - easier to edit and version control than Markdown tables

## Troubleshooting

### Build fails with missing file

Run validation to find broken references:

```bash
sysdoc validate --check-images --check-tables
```

### Diagram doesn't appear

Ensure the diagram is:
- In SVG format (`.drawio.svg` or `.svg`)
- Referenced with correct relative path
- Actually exists at that path

### Table not rendering

Check that:
- CSV file exists at referenced path
- CSV is properly formatted
- Table reference uses correct syntax: `<!-- TABLE: path/to/file.csv -->`

## Next Steps

- Explore the [DI-IPSC-81435B template](../examples/templates/DI-IPSC-81435B/)
- Review the [complete SDD example](../examples/complete-sdd/)
- Check out the [API documentation](api.md) (when available)
- Contribute to sysdoc on [GitHub](https://github.com/yourusername/sysdoc)
