//! Validation error types

use std::path::PathBuf;
use thiserror::Error;

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    /// A referenced image file is missing
    #[error("Missing image '{image_path}' referenced in '{referenced_in}'", image_path = image_path.display(), referenced_in = referenced_in.display())]
    MissingImage {
        referenced_in: PathBuf,
        image_path: PathBuf,
    },
    /// A referenced table file is missing
    #[error("Missing table '{table_path}' referenced in '{referenced_in}'", table_path = table_path.display(), referenced_in = referenced_in.display())]
    MissingTable {
        referenced_in: PathBuf,
        table_path: PathBuf,
    },
    /// A referenced include file is missing
    #[error("Missing include file '{include_path}' referenced in '{referenced_in}'", include_path = include_path.display(), referenced_in = referenced_in.display())]
    MissingIncludeFile {
        referenced_in: PathBuf,
        include_path: PathBuf,
    },
    /// Duplicate section_id found in metadata
    #[error("Duplicate section_id '{section_id}':\n  First occurrence:  {first_location}:{first_line}\n  Second occurrence: {second_location}:{second_line}", first_location = first_location.display(), second_location = second_location.display())]
    DuplicateSectionId {
        section_id: String,
        first_location: PathBuf,
        first_line: usize,
        second_location: PathBuf,
        second_line: usize,
    },
    /// Multiple validation errors
    #[error("Multiple validation errors: {}", format_errors(.0))]
    Multiple(Vec<ValidationError>),
}

/// Helper function to format multiple errors
fn format_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(|e| format!("\n  - {}", e))
        .collect::<String>()
}
