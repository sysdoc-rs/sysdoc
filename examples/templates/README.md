# sysdoc Templates

This directory contains document templates used by the `sysdoc init` command.

## Available Templates

### DI-IPSC-81435B

A complete template based on the DI-IPSC-81435B Data Item Description (DID) for Software Design Description (SDD).

**Structure:**

```text
templates/src/DI-IPSC-81435B/
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

**Usage:**

```bash
sysdoc init DI-IPSC-81435B my-project-name
```

This will create a new directory with the template structure, ready for you to fill in with your project-specific content.

## File Naming Convention

All template files use section number prefixes for organization:

- Format: `XX.YY_descriptive-name.md`
- Example: `01.01_identification.md`, `03.02_architectural-design.md`
- Section folders use numbered prefixes: `01-scope/`, `02-referenced-documents/`, etc.

This convention ensures files are easy to navigate and clearly organized in your IDE.
