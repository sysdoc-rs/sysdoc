# Complete SDD Example

This is a comprehensive Software Design Description (SDD) example demonstrating all sysdoc features.

## Structure

```
complete-sdd/
├── README.md
└── src/
    ├── 01-introduction/
    │   ├── 01.00_introduction.md
    │   └── 01.01_purpose.md
    ├── 02-architecture/
    │   ├── 02.00_architecture.md
    │   ├── 02.01_system-context.md
    │   ├── 02.02_component-architecture.md
    │   ├── 02.03_component-summary.md
    │   ├── diagrams/
    │   │   ├── system-context.drawio.svg
    │   │   └── component-diagram.drawio.svg
    │   └── tables/
    │       └── components.csv
    ├── 03-detailed-design/
    │   ├── 03.00_detailed-design.md
    │   ├── 03.01_ui-component.md
    │   ├── 03.02_data-component.md
    │   └── ui-screenshot.png
    └── 04-interfaces/
        ├── 04.00_interfaces.md
        ├── 04.01_api-endpoints.md
        └── api-endpoints.csv
```

**Note:** All markdown files use section number prefixes (e.g., `01.00_introduction.md`, `02.01_system-context.md`) for better organization and IDE tab clarity.

## Building

```bash
cd src
sysdoc build -o complete-sdd.docx
```

## Features Demonstrated

- Nested folder structure (multi-level sections)
- Section number prefixes in filenames (XX.YY_name.md)
- Auto-nested heading depth
- Multiple DrawIO SVG diagrams
- PNG images
- CSV tables
- Complex document organization
- Source files organized under src/ directory
