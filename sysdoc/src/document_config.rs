//! Document configuration from sysdoc.toml

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Main document configuration from sysdoc.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConfig {
    /// Optional system identifier that this document belongs to
    pub system_id: Option<String>,

    /// Unique identifier for the document
    pub document_id: String,

    /// Human-readable document title
    pub document_title: String,

    /// Optional document subtitle (used for dc:subject in DOCX)
    pub document_subtitle: Option<String>,

    /// Optional document description (used for dc:description in DOCX)
    pub document_description: Option<String>,

    /// Document owner/author information
    pub document_owner: Person,

    /// Document approver information
    pub document_approver: Person,

    /// Type of document (e.g., SSS, SSDD, SDD, SRS, ICD, etc.)
    pub document_type: String,

    /// Standard the document follows (e.g., DI-IPSC-81435B)
    pub document_standard: String,

    /// Template used to create the document
    pub document_template: String,

    /// Path to the .docx file to use as template for generated docx files
    pub docx_template_path: Option<String>,
}

/// Person information (owner, approver, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// Person's full name
    pub name: String,

    /// Person's email address
    pub email: String,
}

impl DocumentConfig {
    /// Load configuration from a sysdoc.toml file
    ///
    /// # Parameters
    /// * `path` - Path to the sysdoc.toml configuration file
    ///
    /// # Returns
    /// * `Ok(DocumentConfig)` - Successfully loaded configuration
    /// * `Err(DocumentConfigError)` - Error reading or parsing the configuration file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, DocumentConfigError> {
        let content = fs::read_to_string(&path).map_err(DocumentConfigError::IoError)?;

        let config: DocumentConfig =
            toml::from_str(&content).map_err(DocumentConfigError::ParseError)?;

        Ok(config)
    }

    /// Save configuration to a sysdoc.toml file
    ///
    /// # Parameters
    /// * `path` - Path where the sysdoc.toml file will be written
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved configuration
    /// * `Err(DocumentConfigError)` - Error serializing or writing the configuration file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), DocumentConfigError> {
        let content = toml::to_string_pretty(self).map_err(DocumentConfigError::SerializeError)?;

        fs::write(&path, content).map_err(DocumentConfigError::IoError)?;

        Ok(())
    }
}

/// Errors that can occur when loading or saving document configuration
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum DocumentConfigError {
    /// IO error when reading or writing file
    IoError(std::io::Error),

    /// Error parsing TOML
    ParseError(toml::de::Error),

    /// Error serializing to TOML
    SerializeError(toml::ser::Error),
}

impl std::fmt::Display for DocumentConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentConfigError::IoError(e) => write!(f, "IO error: {}", e),
            DocumentConfigError::ParseError(e) => write!(f, "TOML parse error: {}", e),
            DocumentConfigError::SerializeError(e) => write!(f, "TOML serialize error: {}", e),
        }
    }
}

impl std::error::Error for DocumentConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_config_roundtrip() {
        let config = DocumentConfig {
            system_id: Some("FCS-2024".to_string()),
            document_id: "SDD-001".to_string(),
            document_title: "Flight Control Software Design Description".to_string(),
            document_subtitle: Some("Avionics Control System".to_string()),
            document_description: Some(
                "Detailed design for the flight control software system".to_string(),
            ),
            document_owner: Person {
                name: "John Doe".to_string(),
                email: "john.doe@example.com".to_string(),
            },
            document_approver: Person {
                name: "Jane Smith".to_string(),
                email: "jane.smith@example.com".to_string(),
            },
            document_type: "SDD".to_string(),
            document_standard: "DI-IPSC-81435B".to_string(),
            document_template: "sdd-standard-v1".to_string(),
            docx_template_path: Some("templates/standard.docx".to_string()),
        };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();
        println!("Generated TOML:\n{}", toml_str);

        // Deserialize back
        let parsed: DocumentConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.document_id, "SDD-001");
        assert_eq!(
            parsed.document_title,
            "Flight Control Software Design Description"
        );
        assert_eq!(parsed.document_owner.name, "John Doe");
        assert_eq!(parsed.document_owner.email, "john.doe@example.com");
        assert_eq!(parsed.document_approver.name, "Jane Smith");
        assert_eq!(parsed.document_approver.email, "jane.smith@example.com");
        assert_eq!(parsed.document_type, "SDD");
        assert_eq!(parsed.document_standard, "DI-IPSC-81435B");
        assert_eq!(parsed.document_template, "sdd-standard-v1");
    }

    #[test]
    fn test_parse_example_toml() {
        let toml_content = r#"
system_id = "SATCOM-2024"
document_id = "SRS-2024-001"
document_title = "Satellite Communication System Requirements"
document_subtitle = "Ground Station Interface"
document_description = "Requirements for satellite ground station communication interface"
document_type = "SRS"
document_standard = "DI-IPSC-81433A"
document_template = "srs-standard-v2"

[document_owner]
name = "Alice Johnson"
email = "alice.johnson@aerospace.com"

[document_approver]
name = "Bob Martinez"
email = "bob.martinez@aerospace.com"
"#;

        let config: DocumentConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.document_id, "SRS-2024-001");
        assert_eq!(config.document_owner.name, "Alice Johnson");
        assert_eq!(config.document_approver.email, "bob.martinez@aerospace.com");
    }
}
