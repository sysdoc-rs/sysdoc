# sysdoc Templates

This directory contains template definitions for initializing new sysdoc documents.

## Available Templates

### Software Design Description (SDD)

**File:** `sdd-standard-v1.toml`
**Standard:** DI-IPSC-81435B
**Sections:**
1. Scope
   - Identification
   - System Overview
   - Document Overview
2. Referenced Documents
3. Software Design Description Overview
4. Software Design
5. Requirements Traceability
6. Notes
7. Appendices

**Usage:**
```bash
sysdoc init --template sdd-standard-v1 --path ./my-sdd
```

### Software Requirements Specification (SRS)

**File:** `srs-standard-v1.toml`
**Standard:** DI-IPSC-81433A
**Sections:**
1. Scope
   - Identification
   - System Overview
   - Document Overview
2. Referenced Documents
3. Requirements
   - Required States and Modes
   - Capability Requirements
   - Interface Requirements
   - Data Requirements
   - Adaptation Requirements
   - Safety Requirements
   - Security and Privacy Requirements
   - Software Environment Requirements
   - Computer Resource Requirements
   - Software Quality Factors
   - Design and Implementation Constraints
   - Personnel-Related Requirements
   - Training-Related Requirements
   - Logistics-Related Requirements
   - Packaging Requirements
4. Qualification Provisions
5. Requirements Traceability
6. Notes

**Usage:**
```bash
sysdoc init --template srs-standard-v1 --path ./my-srs
```

## Template Structure

Each template is defined as a TOML file with the following structure:

```toml
name = "template-name"
document_type = "SDD"
template_spec = "DI-IPSC-81435B"

[files."path/to/file.md"]
heading = "Section Heading"
guidance = """
Guidance text from the standard...
"""
```

See [docs/template-schema.md](../docs/template-schema.md) for complete documentation.

## Creating a New Template

1. Copy an existing template file as a starting point
2. Update the template metadata (`name`, `document_type`, `template_spec`)
3. Modify the file structure and content as needed
4. Add guidance text from the relevant standard
5. Test the template:
   ```bash
   sysdoc init --template your-template-name --path ./test-doc
   ```

## Template Files

Each template creates:
- `sysdoc.toml` - Document configuration
- `README.md` - Repository readme
- `src/` - Markdown source files organized by section
- `.gitignore` - Git ignore file for build outputs

## Standards Reference

| DID Number | Document Type | Full Name |
|------------|---------------|-----------|
| DI-IPSC-81433A | SRS | Software Requirements Specification |
| DI-IPSC-81435B | SDD | Software Design Description |
| DI-IPSC-81436A | IDD/ICD | Interface Design Description / Interface Control Document |
| DI-IPSC-81438A | STD | Software Test Description |
| DI-IPSC-81439A | STR | Software Test Report |
| DI-IPSC-81440A | SUM | Software User Manual |

## Future Templates

Planned templates:
- Interface Control Document (ICD) - DI-IPSC-81436A
- Software Test Plan (STP) - DI-IPSC-81438A
- Software Test Description (STD) - DI-IPSC-81438A
- Software Test Report (STR) - DI-IPSC-81439A
- Software User Manual (SUM) - DI-IPSC-81440A

## Contributing Templates

To contribute a new template:
1. Create the template TOML file following the schema
2. Include accurate guidance from the standard
3. Test the template thoroughly
4. Update this README with the new template information
5. Submit a pull request
