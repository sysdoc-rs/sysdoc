//! Unified document model for the transformation stage (Stage 2)
//!
//! This module defines the structures used after parsing source files
//! and aggregating them into a unified document structure ready for export.

use crate::source_model::{MarkdownSection, TableSource};
use std::path::PathBuf;

/// The unified document model ready for export
#[derive(Debug)]
pub struct UnifiedDocument {
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Root directory of the source
    pub root: PathBuf,
    /// Sorted sections of the document (from all markdown files)
    pub sections: Vec<MarkdownSection>,
    /// All tables used in the document
    pub tables: Vec<TableSource>,
}

impl UnifiedDocument {
    /// Create a new empty unified document
    ///
    /// # Parameters
    /// * `metadata` - Document metadata including title, owner, approver, etc.
    /// * `root` - Root directory path of the document source
    ///
    /// # Returns
    /// * `UnifiedDocument` - A new empty unified document with no sections or tables
    pub fn new(metadata: DocumentMetadata, root: PathBuf) -> Self {
        Self {
            metadata,
            root,
            sections: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Get the total number of tables
    ///
    /// # Returns
    /// * `usize` - Total number of tables in the document
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Get the total number of sections
    ///
    /// # Returns
    /// * `usize` - Total number of sections in the document
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Get the total word count across all sections
    ///
    /// # Returns
    /// * `usize` - Total word count (currently counts content blocks, not actual words)
    pub fn word_count(&self) -> usize {
        // TODO: Implement proper word counting from MarkdownBlock content
        self.sections.iter().map(|s| s.content.len()).sum()
    }

    /// Get the total number of images
    ///
    /// # Returns
    /// * `usize` - Total number of images embedded in all sections
    pub fn image_count(&self) -> usize {
        use crate::source_model::MarkdownBlock;
        self.sections
            .iter()
            .flat_map(|s| &s.content)
            .filter(|block| matches!(block, MarkdownBlock::Image { .. }))
            .count()
    }
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// System identifier (if any)
    pub system_id: Option<String>,
    /// Document unique identifier
    pub document_id: String,
    /// Document title
    pub title: String,
    /// Document subtitle (if any)
    pub subtitle: Option<String>,
    /// Document description (if any)
    pub description: Option<String>,
    /// Document type (SDD, SRS, etc.)
    pub doc_type: String,
    /// Standard/specification
    pub standard: String,
    /// Template used
    pub template: String,
    /// Document owner
    pub owner: Person,
    /// Document approver
    pub approver: Person,
    /// Version number (if any)
    pub version: Option<String>,
    /// Creation date
    pub created: Option<String>,
    /// Last modified date
    pub modified: Option<String>,
}

/// Person information
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    pub email: String,
}

/// Builder for constructing a UnifiedDocument from source models
pub struct DocumentBuilder {
    metadata: DocumentMetadata,
    root: PathBuf,
    sections: Vec<MarkdownSection>,
    tables: Vec<TableSource>,
}

impl DocumentBuilder {
    /// Create a new document builder
    ///
    /// # Parameters
    /// * `metadata` - Document metadata including title, owner, approver, etc.
    /// * `root` - Root directory path of the document source
    ///
    /// # Returns
    /// * `DocumentBuilder` - A new builder with empty sections and tables
    pub fn new(metadata: DocumentMetadata, root: PathBuf) -> Self {
        Self {
            metadata,
            root,
            sections: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Add a section to the document
    ///
    /// # Parameters
    /// * `section` - Markdown section to add
    pub fn add_section(&mut self, section: MarkdownSection) {
        self.sections.push(section);
    }

    /// Add a table to the document
    ///
    /// # Parameters
    /// * `table` - Table source to add
    pub fn add_table(&mut self, table: TableSource) {
        self.tables.push(table);
    }

    /// Build the unified document
    ///
    /// # Returns
    /// * `UnifiedDocument` - The completed unified document with all added content
    pub fn build(self) -> UnifiedDocument {
        UnifiedDocument {
            metadata: self.metadata,
            root: self.root,
            sections: self.sections,
            tables: self.tables,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_model::SectionNumber;

    fn test_metadata() -> DocumentMetadata {
        DocumentMetadata {
            system_id: None,
            document_id: "TEST-001".to_string(),
            title: "Test Document".to_string(),
            subtitle: None,
            description: None,
            doc_type: "SDD".to_string(),
            standard: "DI-IPSC-81435B".to_string(),
            template: "sdd-standard-v1".to_string(),
            owner: Person {
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
            },
            approver: Person {
                name: "Jane Smith".to_string(),
                email: "jane@example.com".to_string(),
            },
            version: None,
            created: None,
            modified: None,
        }
    }

    #[test]
    fn test_document_builder() {
        let section = MarkdownSection {
            heading_level: 1,
            heading_text: "Introduction".to_string(),
            section_number: SectionNumber::parse("1").unwrap(),
            content: vec![],
            metadata: None,
        };

        let mut builder = DocumentBuilder::new(test_metadata(), PathBuf::from("."));
        builder.add_section(section);
        let doc = builder.build();

        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].heading_text, "Introduction");
    }

    #[test]
    fn test_table_count() {
        let doc = UnifiedDocument::new(test_metadata(), PathBuf::from("."));
        assert_eq!(doc.table_count(), 0);
    }
}
