//! Section metadata for traceability
//!
//! This module defines metadata that can be embedded in markdown sections
//! using sysdoc code blocks to support requirements traceability.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

/// Configuration for traceability table generation
///
/// Supports two forms:
/// - `false` - Don't generate a table (default)
/// - `["Header1", "Header2"]` - Generate a table with the specified column headers
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TableGeneration {
    /// Table generation disabled
    #[default]
    Disabled,

    /// Generate table with specified headers (column1, column2)
    Enabled(String, String),
}

impl TableGeneration {
    /// Check if table generation is enabled
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_, _))
    }

    /// Get the column headers for the table
    /// Returns None if disabled
    pub fn get_headers(&self) -> Option<(String, String)> {
        match self {
            Self::Disabled => None,
            Self::Enabled(col1, col2) => Some((col1.clone(), col2.clone())),
        }
    }
}

#[allow(clippy::excessive_nesting)] // Serde visitor pattern requires this nesting
impl<'de> Deserialize<'de> for TableGeneration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TableGenerationVisitor;

        impl<'de> Visitor<'de> for TableGenerationVisitor {
            type Value = TableGeneration;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("false or an array of two strings")
            }

            fn visit_bool<E>(self, value: bool) -> Result<TableGeneration, E>
            where
                E: de::Error,
            {
                if value {
                    Err(de::Error::custom(
                        "table generation requires custom headers: use [\"Header1\", \"Header2\"] instead of true"
                    ))
                } else {
                    Ok(TableGeneration::Disabled)
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<TableGeneration, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let col1 = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::custom("expected two strings in array"))?;
                let col2 = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::custom("expected two strings in array"))?;

                // Ensure no extra elements
                if seq.next_element::<String>()?.is_some() {
                    return Err(de::Error::custom("array must contain exactly two strings"));
                }

                Ok(TableGeneration::Enabled(col1, col2))
            }
        }

        deserializer.deserialize_any(TableGenerationVisitor)
    }
}

/// Metadata for a markdown section, parsed from `sysdoc` code blocks.
///
/// This struct is populated from TOML content within a fenced code block
/// with the `sysdoc` language identifier:
///
/// ```markdown
/// ```sysdoc
/// section_id = "REQ-001"
/// traced_ids = ["SRS-001", "SRS-002"]
/// ```
/// ```
///
/// The metadata enables traceability features like generating tables that map
/// section IDs to traced requirements and vice versa.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SectionMetadata {
    /// Unique identifier for this section (e.g., "REQ-001", "SDD-3.2.1")
    pub section_id: Option<String>,

    /// List of IDs that this section traces to (e.g., requirements IDs)
    pub traced_ids: Option<Vec<String>>,

    /// Configuration for generating a table mapping section_ids to their traced_ids
    ///
    /// Supports:
    /// - `false` - Don't generate (default)
    /// - `["Header1", "Header2"]` - Generate with custom column headers
    ///
    /// The generated table will have:
    /// - First column: section_id (sorted lexically)
    /// - Second column: comma-separated list of traced_ids (sorted lexically)
    pub generate_section_id_to_traced_ids_table: TableGeneration,

    /// Configuration for generating a table mapping traced_ids to section_ids that reference them
    ///
    /// Supports:
    /// - `false` - Don't generate (default)
    /// - `["Header1", "Header2"]` - Generate with custom column headers
    ///
    /// The generated table will have:
    /// - First column: traced_id (deduplicated, sorted lexically)
    /// - Second column: comma-separated list of section_ids (sorted lexically)
    pub generate_traced_ids_to_section_ids_table: TableGeneration,
}

impl SectionMetadata {
    /// Parse metadata from TOML content
    ///
    /// # Parameters
    /// * `content` - TOML string to parse
    ///
    /// # Returns
    /// * `Ok(SectionMetadata)` - Successfully parsed metadata
    /// * `Err(toml::de::Error)` - Parse error
    pub fn parse(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Check if this metadata has any traceability content
    pub fn has_traceability(&self) -> bool {
        self.section_id.is_some() || self.traced_ids.is_some()
    }

    /// Check if this metadata requests any table generation
    pub fn requests_table_generation(&self) -> bool {
        self.generate_section_id_to_traced_ids_table.is_enabled()
            || self.generate_traced_ids_to_section_ids_table.is_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let metadata = SectionMetadata::parse("").unwrap();
        assert_eq!(metadata.section_id, None);
        assert_eq!(metadata.traced_ids, None);
        assert_eq!(
            metadata.generate_section_id_to_traced_ids_table,
            TableGeneration::Disabled
        );
        assert_eq!(
            metadata.generate_traced_ids_to_section_ids_table,
            TableGeneration::Disabled
        );
    }

    #[test]
    fn test_parse_section_id_only() {
        let metadata = SectionMetadata::parse(r#"section_id = "REQ-001""#).unwrap();
        assert_eq!(metadata.section_id, Some("REQ-001".to_string()));
        assert_eq!(metadata.traced_ids, None);
    }

    #[test]
    fn test_parse_traced_ids() {
        let metadata = SectionMetadata::parse(r#"traced_ids = ["SRS-001", "SRS-002"]"#).unwrap();
        assert_eq!(
            metadata.traced_ids,
            Some(vec!["SRS-001".to_string(), "SRS-002".to_string()])
        );
    }

    #[test]
    fn test_parse_full_metadata() {
        let content = r#"
section_id = "SDD-3.2.1"
traced_ids = ["SRS-REQ-001", "SRS-REQ-002"]
generate_section_id_to_traced_ids_table = ["Software Unit", "Software Requirement"]
generate_traced_ids_to_section_ids_table = false
"#;
        let metadata = SectionMetadata::parse(content).unwrap();
        assert_eq!(metadata.section_id, Some("SDD-3.2.1".to_string()));
        assert_eq!(
            metadata.traced_ids,
            Some(vec!["SRS-REQ-001".to_string(), "SRS-REQ-002".to_string()])
        );
        assert_eq!(
            metadata.generate_section_id_to_traced_ids_table,
            TableGeneration::Enabled(
                "Software Unit".to_string(),
                "Software Requirement".to_string()
            )
        );
        assert_eq!(
            metadata.generate_traced_ids_to_section_ids_table,
            TableGeneration::Disabled
        );
    }

    #[test]
    fn test_has_traceability() {
        let empty = SectionMetadata::default();
        assert!(!empty.has_traceability());

        let with_section_id = SectionMetadata {
            section_id: Some("REQ-001".to_string()),
            ..Default::default()
        };
        assert!(with_section_id.has_traceability());

        let with_traced_ids = SectionMetadata {
            traced_ids: Some(vec!["SRS-001".to_string()]),
            ..Default::default()
        };
        assert!(with_traced_ids.has_traceability());
    }

    #[test]
    fn test_requests_table_generation() {
        let empty = SectionMetadata::default();
        assert!(!empty.requests_table_generation());

        let with_forward = SectionMetadata {
            generate_section_id_to_traced_ids_table: TableGeneration::Enabled(
                "A".to_string(),
                "B".to_string(),
            ),
            ..Default::default()
        };
        assert!(with_forward.requests_table_generation());

        let with_reverse = SectionMetadata {
            generate_traced_ids_to_section_ids_table: TableGeneration::Enabled(
                "X".to_string(),
                "Y".to_string(),
            ),
            ..Default::default()
        };
        assert!(with_reverse.requests_table_generation());
    }

    #[test]
    fn test_parse_custom_headers() {
        let content = r#"
generate_section_id_to_traced_ids_table = ["Configuration Item", "System Requirement"]
generate_traced_ids_to_section_ids_table = ["System Requirement", "Configuration Items"]
"#;
        let metadata = SectionMetadata::parse(content).unwrap();
        assert_eq!(
            metadata.generate_section_id_to_traced_ids_table,
            TableGeneration::Enabled(
                "Configuration Item".to_string(),
                "System Requirement".to_string()
            )
        );
        assert_eq!(
            metadata.generate_traced_ids_to_section_ids_table,
            TableGeneration::Enabled(
                "System Requirement".to_string(),
                "Configuration Items".to_string()
            )
        );
    }

    #[test]
    fn test_table_generation_get_headers() {
        let disabled = TableGeneration::Disabled;
        assert_eq!(disabled.get_headers(), None);

        let enabled = TableGeneration::Enabled("Custom1".to_string(), "Custom2".to_string());
        assert_eq!(
            enabled.get_headers(),
            Some(("Custom1".to_string(), "Custom2".to_string()))
        );
    }

    #[test]
    fn test_parse_true_returns_error() {
        let content = r#"
generate_section_id_to_traced_ids_table = true
"#;
        let result = SectionMetadata::parse(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("custom headers"));
    }
}
