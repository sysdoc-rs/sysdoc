# Minimal SDD Example

This is a minimal Software Design Description (SDD) example demonstrating basic sysdoc features.

## Structure

```
minimal-sdd/
├── README.md              (this file)
└── src/
    ├── 01-introduction/
    │   ├── 01.01_purpose.md
    │   ├── 01.02_scope.md
    │   └── 01.03_document-overview.md
    ├── 02-architecture/
    │   ├── 02.01_overview.md
    │   ├── 02.02_system-diagram.md
    │   ├── 02.03_components.md
    │   └── system-diagram.drawio.svg
    └── 03-design/
        ├── 03.01_ui-component.md
        ├── 03.02_business-logic-component.md
        └── 03.03_data-store-component.md
```

**Note:** All markdown files use section number prefixes (e.g., `01.01_purpose.md`) for better organization and IDE tab clarity.

## Building

```bash
cd src
sysdoc build -o minimal-sdd.docx
```

## Features Demonstrated

- Basic folder structure with numbered sections
- Section number prefixes in filenames (XX.YY_name.md)
- Markdown content organized by section
- Single DrawIO diagram
- Auto-generated table of contents
- Source files organized under src/ directory
