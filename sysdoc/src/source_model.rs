//! Source model for the parsing stage
//!
//! This module defines the structures used during Stage 1 (Parsing)
//! where markdown files, images, and CSV files are loaded and validated.

use crate::document_config::DocumentConfig;
use std::path::PathBuf;

// Submodules
mod blocks;
mod image;
mod markdown_source;
mod parser;
mod section_number;
mod table;
mod text_run;
mod types;
mod validation;

// Re-export public types
pub use blocks::MarkdownBlock;
pub use image::{ImageFormat, ImageSource};
pub use markdown_source::{MarkdownSection, MarkdownSource};
pub use section_number::SectionNumber;
pub use table::TableSource;
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
    pub markdown_files: Vec<MarkdownSource>,

    /// All image files referenced in the markdown
    pub image_files: Vec<ImageSource>,

    /// All CSV table files referenced in the markdown
    pub table_files: Vec<TableSource>,
}

impl SourceModel {
    /// Create a new empty source model
    pub fn new(root: PathBuf, config: DocumentConfig) -> Self {
        Self {
            root,
            config,
            markdown_files: Vec::new(),
            image_files: Vec::new(),
            table_files: Vec::new(),
        }
    }

    /// Validate that all referenced resources exist
    pub fn validate(&self) -> Result<(), ValidationError> {
        let image_errors = self.validate_image_references();
        let table_errors = self.validate_table_references();

        let errors: Vec<ValidationError> = image_errors.into_iter().chain(table_errors).collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::Multiple(errors))
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
            .filter_map(|block| {
                if let MarkdownBlock::Image { path, .. } = block {
                    Some(path)
                } else {
                    None
                }
            })
            .filter(|img_path| !self.image_files.iter().any(|img| &img.path == *img_path))
            .map(|img_path| ValidationError::MissingImage {
                referenced_in: md_file.path.clone(),
                image_path: img_path.clone(),
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
            .table_refs
            .iter()
            .filter(|table_ref| !self.table_files.iter().any(|tbl| &tbl.path == *table_ref))
            .map(|table_ref| ValidationError::MissingTable {
                referenced_in: md_file.path.clone(),
                table_path: table_ref.clone(),
            })
            .collect()
    }
}
