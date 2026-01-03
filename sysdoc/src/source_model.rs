//! Source model for the parsing stage
//!
//! This module defines the structures used during Stage 1 (Parsing)
//! where markdown files, images, and CSV files are loaded and validated.

use crate::document_config::DocumentConfig;
use std::path::{Path, PathBuf};

// Submodules
mod blocks;
mod error;
mod image;
mod markdown_source;
mod parser;
mod section_metadata;
mod section_number;
mod table;
mod text_run;
mod types;
mod validation;

// Re-export public types
pub use blocks::{ListItem, MarkdownBlock};
pub use error::SourceModelError;
pub use image::ImageFormat;
pub use markdown_source::{MarkdownSection, MarkdownSource};
pub use section_number::SectionNumber;
pub use table::TableSource;
pub use text_run::TextRun;
pub use types::Alignment;
pub use validation::ValidationError;

/// Collection of all source files discovered and parsed
#[derive(Debug)]
pub struct SourceModel {
    /// Root directory of the document
    pub root: PathBuf,

    /// Document configuration from sysdoc.toml
    pub config: DocumentConfig,

    /// All markdown source files, ordered by discovery (not sorted yet)
    /// CSV tables are embedded as CsvTable blocks within the markdown sections
    pub markdown_files: Vec<MarkdownSource>,
}

impl SourceModel {
    /// Create a new empty source model
    ///
    /// # Parameters
    /// * `root` - Root directory path of the document
    /// * `config` - Document configuration loaded from sysdoc.toml
    ///
    /// # Returns
    /// * `SourceModel` - A new empty source model with no files
    pub fn new(root: PathBuf, config: DocumentConfig) -> Self {
        Self {
            root,
            config,
            markdown_files: Vec::new(),
        }
    }

    /// Validate that all referenced resources exist
    ///
    /// # Returns
    /// * `Ok(())` - All referenced images, tables, and include files exist, and all section_ids are unique
    /// * `Err(ValidationError)` - One or more referenced resources are missing or duplicate section_ids found
    pub fn validate(&self) -> Result<(), ValidationError> {
        let image_errors = self.validate_image_references();
        let table_errors = self.validate_table_references();
        let include_errors = self.validate_include_references();
        let section_id_errors = self.validate_unique_section_ids();

        let errors: Vec<ValidationError> = image_errors
            .into_iter()
            .chain(table_errors)
            .chain(include_errors)
            .chain(section_id_errors)
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }

    /// Generate traceability tables across all files
    ///
    /// This method should be called after validation and after all files are parsed.
    /// It collects traceability data from ALL sections across ALL files, then generates
    /// the requested traceability tables.
    pub fn generate_traceability_tables(&mut self) {
        // Collect all traceability data from all sections across all files
        let section_to_traced = self.collect_all_section_traceability();

        // Build reverse mapping: traced_id -> [section_ids]
        let traced_to_sections = self.build_reverse_traceability(&section_to_traced);

        // Generate tables for sections that request them
        for md_file in self.markdown_files.iter_mut() {
            for section in md_file.sections.iter_mut() {
                Self::generate_tables_for_section(section, &section_to_traced, &traced_to_sections);
            }
        }
    }

    /// Generate traceability tables for a single section if requested
    fn generate_tables_for_section(
        section: &mut MarkdownSection,
        section_to_traced: &[(String, Vec<String>)],
        traced_to_sections: &std::collections::BTreeMap<String, Vec<String>>,
    ) {
        let Some(ref metadata) = section.metadata else {
            return;
        };

        // Generate section_id -> traced_ids table
        if let Some((col1, col2)) = metadata
            .generate_section_id_to_traced_ids_table
            .get_headers()
        {
            let table = Self::create_section_to_traced_table(section_to_traced, &col1, &col2);
            section.content.push(table);
        }

        // Generate traced_id -> section_ids table
        if let Some((col1, col2)) = metadata
            .generate_traced_ids_to_section_ids_table
            .get_headers()
        {
            let table = Self::create_traced_to_sections_table(traced_to_sections, &col1, &col2);
            section.content.push(table);
        }
    }

    /// Collect traceability data from ALL sections across ALL files
    fn collect_all_section_traceability(&self) -> Vec<(String, Vec<String>)> {
        let mut section_to_traced: Vec<(String, Vec<String>)> = self
            .markdown_files
            .iter()
            .flat_map(|md_file| md_file.sections.iter())
            .filter_map(Self::extract_section_traceability)
            .collect();

        // Sort by section_id
        section_to_traced.sort_by(|a, b| a.0.cmp(&b.0));
        section_to_traced
    }

    /// Extract traceability data from a section if it has both section_id and metadata
    fn extract_section_traceability(section: &MarkdownSection) -> Option<(String, Vec<String>)> {
        let metadata = section.metadata.as_ref()?;
        let section_id = metadata.section_id.as_ref()?;
        let traced = metadata.traced_ids.clone().unwrap_or_default();
        Some((section_id.clone(), traced))
    }

    /// Build reverse mapping from traced_id to section_ids
    fn build_reverse_traceability(
        &self,
        section_to_traced: &[(String, Vec<String>)],
    ) -> std::collections::BTreeMap<String, Vec<String>> {
        let mut traced_to_sections: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        for (section_id, traced_ids) in section_to_traced {
            for traced_id in traced_ids {
                traced_to_sections
                    .entry(traced_id.clone())
                    .or_default()
                    .push(section_id.clone());
            }
        }

        // Sort the section_ids within each traced_id entry
        for section_ids in traced_to_sections.values_mut() {
            section_ids.sort();
        }

        traced_to_sections
    }

    /// Create a table mapping section_ids to their traced_ids
    fn create_section_to_traced_table(
        section_to_traced: &[(String, Vec<String>)],
        col1_header: &str,
        col2_header: &str,
    ) -> MarkdownBlock {
        let headers = vec![
            vec![TextRun::new(col1_header.to_string())],
            vec![TextRun::new(col2_header.to_string())],
        ];

        let rows: Vec<Vec<Vec<TextRun>>> = section_to_traced
            .iter()
            .map(|(section_id, traced_ids)| {
                let mut sorted_traced = traced_ids.clone();
                sorted_traced.sort();
                vec![
                    vec![TextRun::new(section_id.clone())],
                    vec![TextRun::new(sorted_traced.join(", "))],
                ]
            })
            .collect();

        MarkdownBlock::InlineTable {
            alignments: vec![Alignment::None, Alignment::None],
            headers,
            rows,
        }
    }

    /// Create a table mapping traced_ids to section_ids that reference them
    fn create_traced_to_sections_table(
        traced_to_sections: &std::collections::BTreeMap<String, Vec<String>>,
        col1_header: &str,
        col2_header: &str,
    ) -> MarkdownBlock {
        let headers = vec![
            vec![TextRun::new(col1_header.to_string())],
            vec![TextRun::new(col2_header.to_string())],
        ];

        let rows: Vec<Vec<Vec<TextRun>>> = traced_to_sections
            .iter()
            .map(|(traced_id, section_ids)| {
                vec![
                    vec![TextRun::new(traced_id.clone())],
                    vec![TextRun::new(section_ids.join(", "))],
                ]
            })
            .collect();

        MarkdownBlock::InlineTable {
            alignments: vec![Alignment::None, Alignment::None],
            headers,
            rows,
        }
    }

    /// Validate all image references
    fn validate_image_references(&self) -> Vec<ValidationError> {
        self.markdown_files
            .iter()
            .flat_map(|md_file| {
                md_file
                    .sections
                    .iter()
                    .flat_map(|section| self.validate_section_images(md_file, section))
            })
            .collect()
    }

    /// Validate image references in a single section
    fn validate_section_images(
        &self,
        md_file: &MarkdownSource,
        section: &MarkdownSection,
    ) -> Vec<ValidationError> {
        section
            .content
            .iter()
            .filter_map(|block| match block {
                MarkdownBlock::Image { path, exists, .. } if !exists => {
                    Some(ValidationError::MissingImage {
                        referenced_in: md_file.path.clone(),
                        image_path: path.clone(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Validate all table references
    fn validate_table_references(&self) -> Vec<ValidationError> {
        self.markdown_files
            .iter()
            .flat_map(|md_file| {
                md_file
                    .sections
                    .iter()
                    .flat_map(|section| self.validate_section_tables(md_file, section))
            })
            .collect()
    }

    /// Validate table references in a single section
    fn validate_section_tables(
        &self,
        md_file: &MarkdownSource,
        section: &MarkdownSection,
    ) -> Vec<ValidationError> {
        section
            .content
            .iter()
            .filter_map(|block| match block {
                MarkdownBlock::CsvTable { path, exists, .. } if !exists => {
                    Some(ValidationError::MissingTable {
                        referenced_in: md_file.path.clone(),
                        table_path: path.clone(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Validate all include file references
    fn validate_include_references(&self) -> Vec<ValidationError> {
        self.markdown_files
            .iter()
            .flat_map(|md_file| {
                md_file
                    .sections
                    .iter()
                    .flat_map(|section| self.validate_section_includes(md_file, section))
            })
            .collect()
    }

    /// Validate include file references in a single section
    fn validate_section_includes(
        &self,
        md_file: &MarkdownSource,
        section: &MarkdownSection,
    ) -> Vec<ValidationError> {
        section
            .content
            .iter()
            .filter_map(|block| match block {
                MarkdownBlock::IncludedCodeBlock { path, exists, .. } if !exists => {
                    Some(ValidationError::MissingIncludeFile {
                        referenced_in: md_file.path.clone(),
                        include_path: path.clone(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Validate that all section_ids are unique across all sections
    fn validate_unique_section_ids(&self) -> Vec<ValidationError> {
        use std::collections::HashMap;

        let mut section_id_locations: HashMap<String, (PathBuf, usize)> = HashMap::new();
        let mut errors = Vec::new();

        for md_file in &self.markdown_files {
            for section in &md_file.sections {
                check_section_id_uniqueness(
                    section,
                    &md_file.path,
                    &mut section_id_locations,
                    &mut errors,
                );
            }
        }

        errors
    }
}

/// Helper function to check if a section_id is unique and record or report duplicates
fn check_section_id_uniqueness(
    section: &MarkdownSection,
    file_path: &Path,
    section_id_locations: &mut std::collections::HashMap<String, (PathBuf, usize)>,
    errors: &mut Vec<ValidationError>,
) {
    // Only check sections that have metadata with a section_id
    let Some(section_id) = section
        .metadata
        .as_ref()
        .and_then(|m| m.section_id.as_ref())
    else {
        return;
    };

    // Check if we've seen this section_id before
    match section_id_locations.get(section_id) {
        Some((first_location, first_line)) => {
            errors.push(ValidationError::DuplicateSectionId {
                section_id: section_id.clone(),
                first_location: first_location.clone(),
                first_line: *first_line,
                second_location: file_path.to_path_buf(),
                second_line: section.line_number,
            });
        }
        None => {
            // First time seeing this section_id, record its location and line number
            section_id_locations.insert(
                section_id.clone(),
                (file_path.to_path_buf(), section.line_number),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_config::Person;
    use crate::source_model::section_metadata::SectionMetadata;

    /// Helper to create a minimal DocumentConfig for testing
    fn test_config() -> DocumentConfig {
        DocumentConfig {
            system_id: None,
            document_id: "TEST-001".to_string(),
            document_title: "Test Document".to_string(),
            document_subtitle: None,
            document_description: None,
            document_owner: Person {
                name: "Test Owner".to_string(),
                email: "owner@test.com".to_string(),
            },
            document_approver: Person {
                name: "Test Approver".to_string(),
                email: "approver@test.com".to_string(),
            },
            document_type: "TEST".to_string(),
            document_standard: "TEST-STANDARD".to_string(),
            document_template: "test-template".to_string(),
            docx_template_path: None,
        }
    }

    #[test]
    fn test_duplicate_section_id_validation() {
        let mut model = SourceModel::new(PathBuf::from("/test"), test_config());

        // Create first markdown file with section_id "REQ-001"
        let file1 = MarkdownSource {
            path: PathBuf::from("file1.md"),
            absolute_path: PathBuf::from("/test/file1.md"),
            section_number: SectionNumber::parse("01").unwrap(),
            title: "File 1".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Section 1".to_string(),
                section_number: SectionNumber::parse("01").unwrap(),
                line_number: 1,
                source_file: PathBuf::from("file1.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-001".to_string()),
                    ..Default::default()
                }),
            }],
        };

        // Create second markdown file with duplicate section_id "REQ-001"
        let file2 = MarkdownSource {
            path: PathBuf::from("file2.md"),
            absolute_path: PathBuf::from("/test/file2.md"),
            section_number: SectionNumber::parse("02").unwrap(),
            title: "File 2".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Section 2".to_string(),
                section_number: SectionNumber::parse("02").unwrap(),
                line_number: 5,
                source_file: PathBuf::from("file2.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-001".to_string()),
                    ..Default::default()
                }),
            }],
        };

        model.markdown_files.push(file1);
        model.markdown_files.push(file2);

        // Validation should fail due to duplicate section_id
        let result = model.validate();
        assert!(result.is_err());

        // Check that the error is about duplicate section_id
        if let Err(ValidationError::Multiple(errors)) = result {
            assert_eq!(errors.len(), 1);
            match &errors[0] {
                ValidationError::DuplicateSectionId {
                    section_id,
                    first_location,
                    first_line,
                    second_location,
                    second_line,
                } => {
                    assert_eq!(section_id, "REQ-001");
                    assert_eq!(first_location, &PathBuf::from("file1.md"));
                    assert_eq!(*first_line, 1);
                    assert_eq!(second_location, &PathBuf::from("file2.md"));
                    assert_eq!(*second_line, 5);
                }
                _ => panic!("Expected DuplicateSectionId error"),
            }
        } else {
            panic!("Expected Multiple validation error");
        }
    }

    #[test]
    fn test_unique_section_ids_pass_validation() {
        let mut model = SourceModel::new(PathBuf::from("/test"), test_config());

        // Create two files with different section_ids
        let file1 = MarkdownSource {
            path: PathBuf::from("file1.md"),
            absolute_path: PathBuf::from("/test/file1.md"),
            section_number: SectionNumber::parse("01").unwrap(),
            title: "File 1".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Section 1".to_string(),
                section_number: SectionNumber::parse("01").unwrap(),
                line_number: 1,
                source_file: PathBuf::from("file1.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-001".to_string()),
                    ..Default::default()
                }),
            }],
        };

        let file2 = MarkdownSource {
            path: PathBuf::from("file2.md"),
            absolute_path: PathBuf::from("/test/file2.md"),
            section_number: SectionNumber::parse("02").unwrap(),
            title: "File 2".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Section 2".to_string(),
                section_number: SectionNumber::parse("02").unwrap(),
                line_number: 1,
                source_file: PathBuf::from("file2.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-002".to_string()), // Different ID
                    ..Default::default()
                }),
            }],
        };

        model.markdown_files.push(file1);
        model.markdown_files.push(file2);

        // Validation should pass with unique section_ids
        let result = model.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_section_id_error_message_format() {
        let mut model = SourceModel::new(PathBuf::from("/test"), test_config());

        // Create first markdown file with section_id "REQ-001" at line 3
        let file1 = MarkdownSource {
            path: PathBuf::from("src/requirements/01.01_functional.md"),
            absolute_path: PathBuf::from("/test/src/requirements/01.01_functional.md"),
            section_number: SectionNumber::parse("01.01").unwrap(),
            title: "Functional Requirements".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Functional Requirements".to_string(),
                section_number: SectionNumber::parse("01.01").unwrap(),
                line_number: 3,
                source_file: PathBuf::from("src/requirements/01.01_functional.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-001".to_string()),
                    ..Default::default()
                }),
            }],
        };

        // Create second markdown file with duplicate section_id "REQ-001" at line 15
        let file2 = MarkdownSource {
            path: PathBuf::from("src/requirements/01.02_nonfunctional.md"),
            absolute_path: PathBuf::from("/test/src/requirements/01.02_nonfunctional.md"),
            section_number: SectionNumber::parse("01.02").unwrap(),
            title: "Non-Functional Requirements".to_string(),
            raw_content: String::new(),
            sections: vec![MarkdownSection {
                heading_level: 1,
                heading_text: "Non-Functional Requirements".to_string(),
                section_number: SectionNumber::parse("01.02").unwrap(),
                line_number: 15,
                source_file: PathBuf::from("src/requirements/01.02_nonfunctional.md"),
                content: Vec::new(),
                metadata: Some(SectionMetadata {
                    section_id: Some("REQ-001".to_string()),
                    ..Default::default()
                }),
            }],
        };

        model.markdown_files.push(file1);
        model.markdown_files.push(file2);

        // Get the error and verify the message format
        let result = model.validate();
        assert!(result.is_err());

        let error_message = format!("{}", result.unwrap_err());
        println!("Error message:\n{}", error_message);

        // Verify the error message includes both file paths with line numbers
        assert!(error_message.contains("src/requirements/01.01_functional.md:3"));
        assert!(error_message.contains("src/requirements/01.02_nonfunctional.md:15"));
        assert!(error_message.contains("REQ-001"));
    }
}
