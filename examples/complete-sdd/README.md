# Complete SDD Example

This is a comprehensive Software Design Description (SDD) example demonstrating all sysdoc features.

## Structure

```
complete-sdd/
├── README.md
├── 01-introduction/
│   ├── introduction.md
│   └── 01-purpose/
│       └── purpose.md
├── 02-architecture/
│   ├── architecture.md
│   ├── diagrams/
│   │   ├── system-context.drawio.svg
│   │   └── component-diagram.drawio.svg
│   └── tables/
│       └── components.csv
├── 03-detailed-design/
│   ├── detailed-design.md
│   ├── 01-ui-component/
│   │   ├── ui-component.md
│   │   └── ui-screenshot.png
│   └── 02-data-component/
│       └── data-component.md
└── 04-interfaces/
    ├── interfaces.md
    └── api-endpoints.csv
```

**Note:** Markdown files use descriptive names (not `index.md`) for better IDE tab clarity.

## Building

```bash
sysdoc build -o complete-sdd.docx
```

## Features Demonstrated

- Nested folder structure (multi-level sections)
- Auto-nested heading depth
- Multiple DrawIO SVG diagrams
- PNG images
- CSV tables
- Complex document organization
