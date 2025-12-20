//! Error types for source model parsing and validation

use thiserror::Error;

/// Errors that can occur during source markdown parsing and validation
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SourceModelError {
    /// Source file has no headings (at least one h1 is required)
    #[error("Source file must contain at least one heading (h1) at the start")]
    NoHeadingFound,

    /// First heading is not level 1
    #[error(
        "First heading must be level 1 (h1), but found level {actual_level} (h{actual_level})"
    )]
    FirstHeadingNotLevel1 {
        /// The actual level of the first heading
        actual_level: usize,
    },

    /// Multiple level 1 headings found (only the first heading may be h1)
    #[error("Only the first heading may be level 1 (h1), but found {count} h1 headings")]
    MultipleLevel1Headings {
        /// Number of h1 headings found
        count: usize,
    },
}
