//! Unified document model for the transformation stage (Stage 2)
//!
//! This module defines the structures used after parsing source files
//! and transforming them into a unified, hierarchical document structure
//! ready for export.

use crate::source_model::{Alignment, ImageSource, SectionNumber, TableSource};
use std::path::PathBuf;

/// Code block kind (for unified document model)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockKind {
    Indented,
    Fenced,
}

/// The unified document model ready for export
#[derive(Debug)]
pub struct UnifiedDocument {
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Root directory of the source
    pub root: PathBuf,
    /// Hierarchical sections of the document
    pub sections: Vec<DocumentSection>,
    /// All images used in the document
    pub images: Vec<ImageSource>,
    /// All tables used in the document
    pub tables: Vec<TableSource>,
}

impl UnifiedDocument {
    /// Create a new empty unified document
    pub fn new(metadata: DocumentMetadata, root: PathBuf) -> Self {
        Self {
            metadata,
            root,
            sections: Vec::new(),
            images: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Get the total word count across all sections
    pub fn word_count(&self) -> usize {
        self.sections.iter().map(|s| s.word_count()).sum()
    }

    /// Get the total number of images
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Get the total number of tables
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document unique identifier
    pub document_id: String,
    /// Document title
    pub title: String,
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

/// A section in the unified document
#[derive(Debug, Clone)]
pub struct DocumentSection {
    /// Section number (e.g., "1.2.3")
    pub number: SectionNumber,
    /// Section title
    pub title: String,
    /// Nesting depth (0 = top level, 1 = first subsection, etc.)
    pub depth: usize,
    /// Adjusted heading level for this section in the final document
    /// This allows nested sections to have properly adjusted heading levels
    pub heading_level: usize,
    /// Content blocks in this section
    pub content: Vec<ContentBlock>,
    /// Subsections (nested sections)
    pub subsections: Vec<DocumentSection>,
}

impl DocumentSection {
    /// Get the word count for this section and all subsections
    pub fn word_count(&self) -> usize {
        let own_count: usize = self.content.iter().map(|block| block.word_count()).sum();
        let subsection_count: usize = self.subsections.iter().map(|s| s.word_count()).sum();
        own_count + subsection_count
    }

    /// Flatten the section hierarchy into a linear list
    pub fn flatten(&self) -> Vec<&DocumentSection> {
        let mut result = vec![self];
        for subsection in &self.subsections {
            result.extend(subsection.flatten());
        }
        result
    }
}

/// A block of content in a section
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// A paragraph of inline content
    Paragraph(Vec<InlineContent>),
    /// A heading (may occur within a section for sub-headings)
    Heading {
        level: usize,
        content: Vec<InlineContent>,
    },
    /// A block quote
    BlockQuote(Vec<ContentBlock>),
    /// A code block
    CodeBlock {
        kind: CodeBlockKind,
        lang: Option<String>,
        code: String,
    },
    /// An ordered or unordered list
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    /// A table
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<Vec<InlineContent>>,
        rows: Vec<Vec<Vec<InlineContent>>>,
    },
    /// An embedded CSV table
    CsvTable {
        path: PathBuf,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// An image
    Image {
        path: PathBuf,
        alt_text: String,
        title: Option<String>,
    },
    /// A horizontal rule
    Rule,
    /// Raw HTML (if needed)
    Html(String),
}

impl ContentBlock {
    /// Get the word count for this content block
    pub fn word_count(&self) -> usize {
        match self {
            ContentBlock::Paragraph(inlines) => inlines.iter().map(|i| i.word_count()).sum(),
            ContentBlock::Heading { content, .. } => content.iter().map(|i| i.word_count()).sum(),
            ContentBlock::BlockQuote(blocks) => blocks.iter().map(|b| b.word_count()).sum(),
            ContentBlock::CodeBlock { code, .. } => code.split_whitespace().count(),
            ContentBlock::List { items, .. } => items.iter().map(|i| i.word_count()).sum(),
            ContentBlock::Table { headers, rows, .. } => {
                let header_count: usize = headers
                    .iter()
                    .flat_map(|h| h.iter().map(|i| i.word_count()))
                    .sum();
                let row_count: usize = rows
                    .iter()
                    .flat_map(|r| r.iter().flat_map(|c| c.iter().map(|i| i.word_count())))
                    .sum();
                header_count + row_count
            }
            ContentBlock::CsvTable { headers, rows, .. } => {
                let header_count: usize =
                    headers.iter().map(|h| h.split_whitespace().count()).sum();
                let row_count: usize = rows
                    .iter()
                    .flat_map(|r| r.iter().map(|c| c.split_whitespace().count()))
                    .sum();
                header_count + row_count
            }
            ContentBlock::Image { alt_text, .. } => alt_text.split_whitespace().count(),
            ContentBlock::Rule | ContentBlock::Html(_) => 0,
        }
    }
}

/// An item in a list
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Content of the list item
    pub content: Vec<ContentBlock>,
    /// Whether this is a checked task list item
    pub checked: Option<bool>,
}

impl ListItem {
    /// Get the word count for this list item
    pub fn word_count(&self) -> usize {
        self.content.iter().map(|b| b.word_count()).sum()
    }
}

/// Inline content within a paragraph or heading
#[derive(Debug, Clone)]
pub enum InlineContent {
    /// Plain text
    Text(String),
    /// Inline code
    Code(String),
    /// Emphasized text (italic)
    Emphasis(Vec<InlineContent>),
    /// Strong text (bold)
    Strong(Vec<InlineContent>),
    /// Strikethrough text
    Strikethrough(Vec<InlineContent>),
    /// A hyperlink
    Link {
        url: String,
        title: Option<String>,
        content: Vec<InlineContent>,
    },
    /// An inline image
    Image {
        path: PathBuf,
        alt_text: String,
        title: Option<String>,
    },
    /// Line break (soft)
    SoftBreak,
    /// Line break (hard)
    HardBreak,
    /// Footnote reference
    FootnoteReference(String),
    /// Raw HTML inline
    Html(String),
}

impl InlineContent {
    /// Get the word count for this inline content
    pub fn word_count(&self) -> usize {
        match self {
            InlineContent::Text(text) => text.split_whitespace().count(),
            InlineContent::Code(code) => code.split_whitespace().count(),
            InlineContent::Emphasis(content)
            | InlineContent::Strong(content)
            | InlineContent::Strikethrough(content) => content.iter().map(|i| i.word_count()).sum(),
            InlineContent::Link { content, .. } => content.iter().map(|i| i.word_count()).sum(),
            InlineContent::Image { alt_text, .. } => alt_text.split_whitespace().count(),
            InlineContent::SoftBreak
            | InlineContent::HardBreak
            | InlineContent::FootnoteReference(_)
            | InlineContent::Html(_) => 0,
        }
    }
}

/// Builder for constructing a UnifiedDocument from source models
pub struct DocumentBuilder {
    metadata: DocumentMetadata,
    root: PathBuf,
    sections: Vec<DocumentSection>,
    images: Vec<ImageSource>,
    tables: Vec<TableSource>,
}

impl DocumentBuilder {
    /// Create a new document builder
    pub fn new(metadata: DocumentMetadata, root: PathBuf) -> Self {
        Self {
            metadata,
            root,
            sections: Vec::new(),
            images: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Add a section to the document
    pub fn add_section(&mut self, section: DocumentSection) {
        self.sections.push(section);
    }

    /// Add an image to the document
    pub fn add_image(&mut self, image: ImageSource) {
        self.images.push(image);
    }

    /// Add a table to the document
    pub fn add_table(&mut self, table: TableSource) {
        self.tables.push(table);
    }

    /// Build the unified document
    pub fn build(self) -> UnifiedDocument {
        UnifiedDocument {
            metadata: self.metadata,
            root: self.root,
            sections: self.sections,
            images: self.images,
            tables: self.tables,
        }
    }
}

// TODO: Rewrite ContentTransformer to work with new Block structure
// The new source model uses Block with TextRun instead of the old MarkdownContent
//
// /// Transformer for converting source Blocks to ContentBlock/InlineContent
// pub struct ContentTransformer;
//
// impl ContentTransformer {
//     /// Transform a sequence of source Blocks into ContentBlocks
//     pub fn transform(blocks: &[SourceBlock]) -> Vec<ContentBlock> {
//         // Implementation needed to convert from SourceBlock to ContentBlock
//         Vec::new()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_word_count() {
        let metadata = DocumentMetadata {
            document_id: "TEST-001".to_string(),
            title: "Test Document".to_string(),
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
        };

        let section = DocumentSection {
            number: SectionNumber { parts: vec![1] },
            title: "Introduction".to_string(),
            depth: 0,
            heading_level: 1,
            content: vec![ContentBlock::Paragraph(vec![InlineContent::Text(
                "This is a test paragraph with several words".to_string(),
            )])],
            subsections: vec![],
        };

        let mut doc = UnifiedDocument::new(metadata, PathBuf::from("."));
        doc.sections.push(section);

        assert_eq!(doc.word_count(), 8); // "This is a test paragraph with several words"
    }

    #[test]
    fn test_section_flatten() {
        let subsection = DocumentSection {
            number: SectionNumber { parts: vec![1, 1] },
            title: "Subsection".to_string(),
            depth: 1,
            heading_level: 2,
            content: vec![],
            subsections: vec![],
        };

        let section = DocumentSection {
            number: SectionNumber { parts: vec![1] },
            title: "Section".to_string(),
            depth: 0,
            heading_level: 1,
            content: vec![],
            subsections: vec![subsection],
        };

        let flattened = section.flatten();
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].title, "Section");
        assert_eq!(flattened[1].title, "Subsection");
    }
}
