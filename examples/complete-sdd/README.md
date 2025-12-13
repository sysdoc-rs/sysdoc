# Complete SDD Example

This is a comprehensive Software Design Description (SDD) example demonstrating all sysdoc features.

## Structure

```
complete-sdd/
├── README.md
├── 01-introduction/
│   ├── index.md
│   └── 01-purpose/
│       └── index.md
├── 02-architecture/
│   ├── index.md
│   ├── diagrams/
│   │   ├── system-context.drawio.svg
│   │   └── component-diagram.drawio.svg
│   └── tables/
│       └── components.csv
├── 03-detailed-design/
│   ├── index.md
│   ├── 01-ui-component/
│   │   ├── index.md
│   │   └── ui-screenshot.png
│   └── 02-data-component/
│       └── index.md
└── 04-interfaces/
    ├── index.md
    └── api-endpoints.csv
```

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
