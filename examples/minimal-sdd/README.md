# Minimal SDD Example

This is a minimal Software Design Description (SDD) example demonstrating basic sysdoc features.

## Structure

```
minimal-sdd/
├── README.md              (this file)
├── 01-introduction/
│   └── introduction.md
├── 02-architecture/
│   ├── architecture.md
│   └── system-diagram.drawio.svg
└── 03-design/
    └── design.md
```

**Note:** Markdown files use descriptive names (not `index.md`) for better IDE tab clarity.

## Building

```bash
sysdoc build -o minimal-sdd.docx
```

## Features Demonstrated

- Basic folder structure with numbered sections
- Markdown content
- Single DrawIO diagram
- Auto-generated table of contents
