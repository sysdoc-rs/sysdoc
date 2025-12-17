//! Validation error types

use std::path::PathBuf;

/// Validation errors
#[derive(Debug)]
pub enum ValidationError {
    /// A referenced image file is missing
    MissingImage {
        referenced_in: PathBuf,
        image_path: PathBuf,
    },
    /// A referenced table file is missing
    MissingTable {
        referenced_in: PathBuf,
        table_path: PathBuf,
    },
    /// Multiple validation errors
    Multiple(Vec<ValidationError>),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingImage {
                referenced_in,
                image_path,
            } => {
                write!(
                    f,
                    "Missing image '{}' referenced in '{}'",
                    image_path.display(),
                    referenced_in.display()
                )
            }
            ValidationError::MissingTable {
                referenced_in,
                table_path,
            } => {
                write!(
                    f,
                    "Missing table '{}' referenced in '{}'",
                    table_path.display(),
                    referenced_in.display()
                )
            }
            ValidationError::Multiple(errors) => {
                writeln!(f, "Multiple validation errors:")?;
                for error in errors {
                    writeln!(f, "  - {}", error)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ValidationError {}
