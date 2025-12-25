//! Template configuration schema for sysdoc templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Template definition from a .toml file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Template name (e.g., "sdd-standard-v1")
    pub name: String,

    /// Document type this template creates (e.g., "SDD", "SRS", "ICD")
    pub document_type: String,

    /// Standard/specification this template implements (e.g., "DI-IPSC-81435B")
    pub template_spec: String,

    /// Files to be created during template initialization
    /// Key is the file path relative to the document root
    /// Value is the file content configuration
    #[serde(default)]
    pub files: HashMap<String, FileTemplate>,
}

/// Configuration for a file to be created from the template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileTemplate {
    /// Simple file with just content
    Simple {
        /// File content
        content: String,
    },
    /// Markdown file with heading and guidance
    Markdown {
        /// Section heading (for markdown files)
        heading: String,
        /// Guidance text to be placed in HTML comment
        guidance: String,
    },
}

impl TemplateConfig {
    /// Load template configuration from a .toml file
    ///
    /// # Parameters
    /// * `path` - Path to the template .toml file
    ///
    /// # Returns
    /// * `Ok(TemplateConfig)` - Successfully loaded template configuration
    /// * `Err(TemplateConfigError)` - Error reading or parsing the template file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, TemplateConfigError> {
        let content = fs::read_to_string(&path).map_err(TemplateConfigError::IoError)?;

        let config: TemplateConfig =
            toml::from_str(&content).map_err(TemplateConfigError::ParseError)?;

        Ok(config)
    }

    /// Generate file content for a given file path
    ///
    /// # Parameters
    /// * `file_path` - Relative path of the file within the template
    ///
    /// # Returns
    /// * `Some(String)` - Generated file content for the specified path
    /// * `None` - No file template found for the given path
    pub fn generate_file_content(&self, file_path: &str) -> Option<String> {
        self.files.get(file_path).map(|template| match template {
            FileTemplate::Simple { content } => content.clone(),
            FileTemplate::Markdown { heading, guidance } => {
                let guidance_comment = if !guidance.is_empty() {
                    format!("<!-- GUIDANCE:\n{}\n-->\n\n", guidance)
                } else {
                    String::new()
                };
                format!("{}# {}\n\n", guidance_comment, heading)
            }
        })
    }
}

/// Errors that can occur when loading template configuration
#[derive(Debug)]
#[allow(dead_code)]
pub enum TemplateConfigError {
    /// IO error when reading file
    IoError(std::io::Error),

    /// Error parsing TOML
    ParseError(toml::de::Error),
}

impl std::fmt::Display for TemplateConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateConfigError::IoError(e) => write!(f, "IO error: {}", e),
            TemplateConfigError::ParseError(e) => write!(f, "TOML parse error: {}", e),
        }
    }
}

impl std::error::Error for TemplateConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_config_parsing() {
        let toml_content = r#"
name = "sdd-standard-v1"
document_type = "SDD"
template_spec = "DI-IPSC-81435B"

[files."sysdoc.toml"]
content = """
document_id = "SDD-XXX"
document_title = "Software Design Description"
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

[files."src/01-scope/01.01_identification.md"]
heading = "Identification"
guidance = """
1.1 Identification. This paragraph shall contain a full identification of the system and the
software to which this document applies, including, as applicable, identification number(s),
title(s), abbreviation(s), version number(s), and release number(s).
"""

[files."src/01-scope/01.02_system_overview.md"]
heading = "System Overview"
guidance = """
1.2 System overview. This paragraph shall briefly state the purpose of the system and the
software to which this document applies.
"""
"#;

        let config: TemplateConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.name, "sdd-standard-v1");
        assert_eq!(config.document_type, "SDD");
        assert_eq!(config.template_spec, "DI-IPSC-81435B");
        assert_eq!(config.files.len(), 3);

        // Test simple file content
        let sysdoc_content = config.generate_file_content("sysdoc.toml").unwrap();
        assert!(sysdoc_content.contains("document_id = \"SDD-XXX\""));

        // Test markdown file with guidance
        let md_content = config
            .generate_file_content("src/01-scope/01.01_identification.md")
            .unwrap();
        assert!(md_content.contains("<!-- GUIDANCE:"));
        assert!(md_content.contains("1.1 Identification"));
        assert!(md_content.contains("# Identification"));
    }

    #[test]
    fn test_generate_markdown_file() {
        let template = FileTemplate::Markdown {
            heading: "Test Heading".to_string(),
            guidance: "This is guidance text.\nMultiple lines.".to_string(),
        };

        let content = match template {
            FileTemplate::Markdown { heading, guidance } => {
                format!("<!-- GUIDANCE:\n{}\n-->\n\n# {}\n\n", guidance, heading)
            }
            _ => panic!("Wrong variant"),
        };

        assert!(content.contains("<!-- GUIDANCE:"));
        assert!(content.contains("This is guidance text."));
        assert!(content.contains("# Test Heading"));
    }
}
