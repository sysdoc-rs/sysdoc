# Minimal SDD Example

This is a minimal Software Design Description (SDD) example demonstrating basic sysdoc features.

## Structure

```
minimal-sdd/
├── README.md              (this file)
├── 01-introduction/
│   └── index.md
├── 02-architecture/
│   ├── index.md
│   └── system-diagram.drawio.svg
└── 03-design/
    └── index.md
```

## Building

```bash
sysdoc build -o minimal-sdd.docx
```

## Features Demonstrated

- Basic folder structure with numbered sections
- Markdown content
- Single DrawIO diagram
- Auto-generated table of contents
