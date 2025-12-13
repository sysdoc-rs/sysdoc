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
└── src/
    ├── 01-scope/
    │   ├── 01.01_identification.md
    │   ├── 01.02_system-overview.md
    │   └── 01.03_document-overview.md
    ├── 02-referenced-documents/
    │   ├── 02.01_applicable-documents.md
    │   └── 02.02_reference-documents.md
    ├── 03-software-design/
    │   ├── 03.00_software-design.md
    │   ├── 03.01_system-wide-design.md
    │   ├── 03.02_architectural-design.md
    │   └── 03.03_detailed-design.md
    ├── 04-requirements-traceability/
    │   └── 04.01_traceability.md
    └── 05-notes/
        ├── 05.01_assumptions.md
        ├── 05.02_open-issues.md
        └── 05.03_future-enhancements.md
```

**Note:** Files use section number prefixes (e.g., `01.01_identification.md`) for organization and easy identification in your IDE tabs. All source files are under the `src/` directory.

### 3. Edit Content

Edit the Markdown files to add your content:

```bash
code src/01-scope/01.01_identification.md
```

Add content following Markdown syntax:

```markdown
# Identification

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

Generate a .docx file (default format):

```bash
cd src
sysdoc build -o flight-control-sdd.docx
```

Or build to consolidated markdown:

```bash
cd src
sysdoc build --format markdown --output ../output
```

With options:

```bash
sysdoc build src --format docx --output deliverables/sdd-v1.0.docx --verbose
```

Or from within the src directory:

```bash
cd src
sysdoc build --output ../deliverables/sdd-v1.0.docx --verbose
```

**Output formats:**

- **DOCX** - Single Word document with embedded images (default)
- **Markdown** - Consolidated markdown file with images in a separate folder

### 7. Validate the Document

Check for broken links and missing files:

```bash
sysdoc validate src --check-links --check-images --check-tables
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
mkdir -p src/01-intro src/02-content
echo "# Introduction" > src/01-intro/01.01_overview.md
sysdoc build src -o my-doc.docx
```

**Tip:** Use section number prefixes (e.g., `01.01_overview.md`) for better clarity when working with multiple files in your editor.

### Nested Sections

sysdoc automatically adjusts heading levels based on folder depth:

```
src/
  01-section/
    01.00_section.md           # H1: Section
    01.01_subsection.md        # H2: Subsection
    01.02_another-subsection/
      01.02.01_details.md      # H3: Sub-subsection
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
cd examples/minimal-sdd/src
sysdoc build -o output.docx
```

## Tips and Best Practices

1. **Use numbered folders** for ordered sections (01-, 02-, etc.)
2. **Use section number prefixes** in filenames (e.g., `01.01_purpose.md`) for organization
3. **Keep all source files in src/** directory for clean repository structure
4. **Keep images close to content** in the same section folder
5. **Use descriptive filenames** for diagrams and tables
6. **Commit often** to Git for version control
7. **Use PR reviews** for collaborative document development
8. **Validate regularly** to catch broken references early
9. **Keep diagrams simple** - complex diagrams don't render well in Word
10. **Use CSV for tables** - easier to edit and version control than Markdown tables

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

- Explore the [DI-IPSC-81435B template](../examples/templates/src/DI-IPSC-81435B/)
- Review the [complete SDD example](../examples/complete-sdd/src/)
- Check out the [API documentation](api.md) (when available)
- Contribute to sysdoc on [GitHub](https://github.com/yourusername/sysdoc)
