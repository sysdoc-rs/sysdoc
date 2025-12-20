//! Embedded template definitions
//!
//! This module contains all built-in templates compiled into the binary.

use crate::template_config::TemplateConfig;
use std::collections::HashMap;

/// Template metadata for display and lookup
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// Template identifier (e.g., "sdd-standard-v1")
    pub id: String,
    /// Document type (e.g., "SDD")
    pub doc_type: String,
    /// Standard specification (e.g., "DI-IPSC-81435B")
    pub spec: String,
    /// Template TOML content
    pub content: &'static str,
}

/// Get all available templates
///
/// # Returns
/// * `Vec<TemplateInfo>` - Vector of all built-in template definitions
pub fn get_all_templates() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            id: "sdd-standard-v1".to_string(),
            doc_type: "SDD".to_string(),
            spec: "DI-IPSC-81435B".to_string(),
            content: include_str!("templates/sdd-standard-v1.toml"),
        },
        TemplateInfo {
            id: "srs-standard-v1".to_string(),
            doc_type: "SRS".to_string(),
            spec: "DI-IPSC-81433A".to_string(),
            content: include_str!("templates/srs-standard-v1.toml"),
        },
        TemplateInfo {
            id: "ssdd-standard-v1".to_string(),
            doc_type: "SSDD".to_string(),
            spec: "DI-IPSC-81437A".to_string(),
            content: include_str!("templates/ssdd-standard-v1.toml"),
        },
        TemplateInfo {
            id: "sss-standard-v1".to_string(),
            doc_type: "SSS".to_string(),
            spec: "DI-IPSC-81431A".to_string(),
            content: include_str!("templates/sss-standard-v1.toml"),
        },
    ]
}

/// Get a template by ID or alias
///
/// # Parameters
/// * `id` - Template identifier, document type (e.g., "SDD"), or specification ID (e.g., "DI-IPSC-81435B")
///
/// # Returns
/// * `Some(TemplateInfo)` - Template information if found
/// * `None` - No template found matching the given identifier
pub fn get_template(id: &str) -> Option<TemplateInfo> {
    // Build a lookup map with both IDs and common aliases
    let mut lookup: HashMap<String, TemplateInfo> = HashMap::new();

    for template in get_all_templates() {
        // Add by full ID
        lookup.insert(template.id.clone(), template.clone());

        // Add common aliases (just the document type)
        lookup.insert(template.doc_type.to_lowercase(), template.clone());
        lookup.insert(template.doc_type.clone(), template.clone());

        // Add by spec ID
        lookup.insert(template.spec.clone(), template.clone());
    }

    lookup.get(id).cloned()
}

/// Parse a template into TemplateConfig
///
/// # Parameters
/// * `template_info` - Template information containing TOML content to parse
///
/// # Returns
/// * `Ok(TemplateConfig)` - Successfully parsed template configuration
/// * `Err(toml::de::Error)` - Error parsing TOML content
pub fn parse_template(template_info: &TemplateInfo) -> Result<TemplateConfig, toml::de::Error> {
    toml::from_str(template_info.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_load() {
        let templates = get_all_templates();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn test_get_template_by_id() {
        assert!(get_template("sdd-standard-v1").is_some());
        assert!(get_template("srs-standard-v1").is_some());
        assert!(get_template("ssdd-standard-v1").is_some());
        assert!(get_template("sss-standard-v1").is_some());
    }

    #[test]
    fn test_get_template_by_alias() {
        assert!(get_template("SDD").is_some());
        assert!(get_template("sdd").is_some());
        assert!(get_template("SRS").is_some());
        assert!(get_template("srs").is_some());
        assert!(get_template("SSDD").is_some());
        assert!(get_template("SSS").is_some());
    }

    #[test]
    fn test_get_template_by_spec() {
        assert!(get_template("DI-IPSC-81435B").is_some());
        assert!(get_template("DI-IPSC-81433A").is_some());
        assert!(get_template("DI-IPSC-81437A").is_some());
        assert!(get_template("DI-IPSC-81431A").is_some());
    }

    #[test]
    fn test_parse_templates() {
        for template in get_all_templates() {
            let parsed = parse_template(&template);
            assert!(parsed.is_ok(), "Failed to parse template: {}", template.id);
        }
    }

    #[test]
    fn test_unknown_template() {
        assert!(get_template("unknown-template").is_none());
    }
}
