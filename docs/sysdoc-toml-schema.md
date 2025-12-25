# sysdoc.toml Schema

The `sysdoc.toml` file contains metadata about a systems engineering document. This file should be placed at the root of your document repository, next to the `src/` folder.

## Schema Definition

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `system_id` | String | No | Optional system identifier (used in DOCX cp:keywords field) |
| `document_id` | String | Yes | Unique identifier for the document (e.g., "SDD-001", "SRS-2024-001") |
| `document_title` | String | Yes | Human-readable title of the document |
| `document_subtitle` | String | No | Optional subtitle (used for dc:subject in DOCX) |
| `document_description` | String | No | Optional description (used for dc:description in DOCX) |
| `document_type` | String | Yes | Type of document (SSS, SSDD, SDD, SRS, ICD, STP, STD, STR, etc.) |
| `document_standard` | String | Yes | Standard or DID the document follows (e.g., "DI-IPSC-81435B") |
| `document_template` | String | Yes | Template used to create the document (for tracking purposes) |
| `document_owner` | Person | Yes | Document owner/author information |
| `document_approver` | Person | Yes | Document approver information |

### Person Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Person's full name |
| `email` | String | Yes | Person's email address |

## Example

```toml
system_id = "FCS-2024"
document_id = "SDD-001"
document_title = "Flight Control Software Design Description"
document_subtitle = "Avionics Control System"
document_description = "Detailed design documentation for the flight control software system"
document_type = "SDD"
document_standard = "DI-IPSC-81435B"
document_template = "sdd-standard-v1"

[document_owner]
name = "John Doe"
email = "john.doe@example.com"

[document_approver]
name = "Jane Smith"
email = "jane.smith@example.com"
```

## Common Document Types

| Code | Full Name |
|------|-----------|
| SSS | System/Subsystem Specification |
| SSDD | System/Subsystem Design Description |
| SDD | Software Design Description |
| SRS | Software Requirements Specification |
| ICD | Interface Control Document |
| STP | Software Test Plan |
| STD | Software Test Description |
| STR | Software Test Report |
| SUM | Software User Manual |
| SPS | Software Product Specification |

## Common Standards (DIDs)

| DID Number | Document Type |
|------------|---------------|
| DI-IPSC-81433A | Software Requirements Specification (SRS) |
| DI-IPSC-81435B | Software Design Description (SDD) |
| DI-IPSC-81436A | Interface Design Description (IDD/ICD) |
| DI-IPSC-81438A | Software Test Description (STD) |
| DI-IPSC-81439A | Software Test Report (STR) |
| DI-IPSC-81440A | Software User Manual (SUM) |

## Usage in Code

```rust
use sysdoc::document_config::DocumentConfig;

// Load configuration
let config = DocumentConfig::load("sysdoc.toml")?;

println!("Document: {} ({})", config.document_title, config.document_id);
println!("Owner: {} <{}>", config.document_owner.name, config.document_owner.email);

// Save configuration
config.save("sysdoc.toml")?;
```

## Validation

The schema enforces:

- Most fields are required, except `system_id`, `document_subtitle`, and `document_description` which are optional
- Email addresses should be valid (enforced by application logic, not schema)
- Document IDs should be unique within your organization (enforced by process, not schema)

## DOCX Metadata Mapping

The configuration fields are mapped to DOCX metadata as follows:

- `document_title` → `<dc:title>`
- `document_subtitle` → `<dc:subject>` (empty if not provided)
- `document_description` → `<dc:description>` (empty if not provided)
- `system_id`, `document_id`, `document_type`, `document_standard` → `<cp:keywords>` (comma-separated)
- `document_owner.name` → `<dc:creator>`
