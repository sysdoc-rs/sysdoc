# sysdoc Templates

This directory contains templates used by the `sysdoc init` command.

## Available Templates

### DI-IPSC-81435B (SDD)
Software Design Description template based on DI-IPSC-81435B standard.

**Aliases:** `SDD`, `sdd`

### Usage

```bash
# Using full DID identifier
sysdoc init DI-IPSC-81435B my-project

# Using alias
sysdoc init SDD my-project
```

## Template Structure

Each template contains:
- Folder structure matching the document outline
- Placeholder `index.md` files with section headings
- README explaining the template
- Example diagrams and tables where appropriate

## Creating Custom Templates

Templates are simply directory structures with Markdown files. To create a custom template:

1. Create a new directory in `examples/templates/`
2. Add the folder structure and placeholder files
3. Add a README.md explaining the template
4. Update the template registry in sysdoc code
