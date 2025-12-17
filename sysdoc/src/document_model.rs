//! Document model for representing parsed markdown documents

use std::path::PathBuf;

use crate::document_section::DocumentSection;

/// Represents the entire document being built
#[derive(Debug)]
pub struct DocumentModel {
    /// Root directory of the document source
    #[allow(dead_code)]
    pub root: PathBuf,
    /// Ordered sections of the document
    pub sections: Vec<DocumentSection>,
}

impl DocumentModel {
    /// Create a new empty document
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            sections: Vec::new(),
        }
    }
}
