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
    /// Duplicate section_id found in metadata
    #[error("Duplicate section_id '{section_id}' found in '{first_location}' and '{second_location}'", first_location = first_location.display(), second_location = second_location.display())]
    DuplicateSectionId {
        section_id: String,
        first_location: PathBuf,
        second_location: PathBuf,
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
