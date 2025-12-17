# Template Definition Schema

Template definitions are TOML files that define how to initialize a new sysdoc document. Each template specifies the document type, standard it follows, and all files to be created.

## Schema Definition

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Template identifier (e.g., "sdd-standard-v1") |
| `document_type` | String | Yes | Document type created by this template (e.g., "SDD", "SRS") |
| `template_spec` | String | Yes | Standard/specification implemented (e.g., "DI-IPSC-81435B") |
| `files` | Table | Yes | Map of file paths to file templates |

### File Template Types

Files can be defined in two ways:

#### 1. Simple Content Files

For files like `sysdoc.toml`, `.gitignore`, `README.md`:

```toml
[files."sysdoc.toml"]
content = """
document_id = "SDD-XXX"
document_name = "Software Design Description"
...
"""
```

#### 2. Markdown Files with Guidance

For section markdown files that should include guidance comments:

```toml
[files."src/01-scope/01.01_identification.md"]
heading = "Identification"
guidance = """
1.1 Identification. This paragraph shall contain a full identification of the system and the
software to which this document applies, including, as applicable, identification number(s),
title(s), abbreviation(s), version number(s), and release number(s).
"""
```

This generates:
```markdown
<!-- GUIDANCE:
1.1 Identification. This paragraph shall contain a full identification of the system and the
software to which this document applies, including, as applicable, identification number(s),
title(s), abbreviation(s), version number(s), and release number(s).
-->

# Identification

```

## Complete Example

```toml
name = "sdd-standard-v1"
document_type = "SDD"
template_spec = "DI-IPSC-81435B"

# Configuration file
[files."sysdoc.toml"]
content = """
document_id = "SDD-XXX"
document_name = "Software Design Description"
document_type = "SDD"
document_standard = "DI-IPSC-81435B"
document_template = "sdd-standard-v1"

[document_owner]
name = "Your Name"
email = "your.email@example.com"

[document_approver]
name = "Approver Name"
email = "approver@example.com"
"""

# Markdown section with guidance
[files."src/01-scope/01.01_identification.md"]
heading = "Identification"
guidance = """
1.1 Identification. This paragraph shall contain a full identification of the system and the
software to which this document applies, including, as applicable, identification number(s),
title(s), abbreviation(s), version number(s), and release number(s).
"""

# Simple text file
[files.".gitignore"]
content = """
build/
*.docx
*.pdf
"""
```

## File Path Conventions

- Use forward slashes `/` for path separators (works on all platforms)
- Paths are relative to the document root
- Directory structure is created automatically from file paths
- Use meaningful section numbering (e.g., `01-scope`, `02-referenced_documents`)

## Usage in Code

```rust
use sysdoc::template_config::TemplateConfig;

// Load template
let template = TemplateConfig::load("templates/sdd-standard-v1.toml")?;

// Get file content
let content = template.generate_file_content("src/01-scope/01.01_identification.md");

// Iterate all files
for (path, file_template) in &template.files {
    let content = template.generate_file_content(path).unwrap();
    // Write file...
}
```

## Creating New Templates

1. Create a new `.toml` file in the `templates/` directory
2. Define the template metadata (`name`, `document_type`, `template_spec`)
3. Add all files that should be created during initialization
4. For markdown sections, include guidance from the standard
5. Test the template by running `sysdoc init --template your-template-name`

## Standard Templates

| Template Name | Document Type | Standard |
|---------------|---------------|----------|
| `sdd-standard-v1` | SDD | DI-IPSC-81435B |
| `srs-standard-v1` | SRS | DI-IPSC-81433A |
| `icd-standard-v1` | ICD | DI-IPSC-81436A |

## Best Practices

1. **Include Guidance**: Always include guidance from the standard as HTML comments in markdown files
2. **Pre-configure sysdoc.toml**: Set reasonable defaults in the template's sysdoc.toml
3. **Add .gitignore**: Always include a .gitignore file to exclude build outputs
4. **Create Directory Structure**: Organize sections into logical directories
5. **Use Standard Numbering**: Follow the numbering scheme from the DID/standard
6. **Add README**: Include a README.md explaining how to use the document repository

## Notes

- Guidance text is wrapped in `<!-- GUIDANCE: ... -->` HTML comments
- HTML comments are invisible when rendered but available in source
- This allows guidance to be present without cluttering the final document
- Users can read guidance in VS Code or any text editor
- Guidance can be removed before final delivery if desired
