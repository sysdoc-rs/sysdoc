//! Block-level markdown elements
//!
//! This module defines the structured representation of markdown content
//! at the block level (paragraphs, headings, lists, code blocks, etc.)

use super::text_run::TextRun;
use super::types::Alignment;
use std::path::PathBuf;

/// Block-level markdown element
#[derive(Debug, Clone)]
pub enum MarkdownBlock {
    /// A heading with level and formatted text
    Heading { level: usize, runs: Vec<TextRun> },

    /// A paragraph of formatted text
    Paragraph(Vec<TextRun>),

    /// An image reference
    Image {
        path: PathBuf,
        alt_text: String,
        title: String,
    },

    /// A code block
    CodeBlock {
        language: Option<String>,
        code: String,
        fenced: bool,
    },

    /// A block quote containing other blocks
    BlockQuote(Vec<MarkdownBlock>),

    /// An ordered or unordered list
    List {
        /// Starting number for ordered lists, None for unordered
        start: Option<u64>,
        items: Vec<ListItem>,
    },

    /// A table
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<Vec<TextRun>>,
        rows: Vec<Vec<Vec<TextRun>>>,
    },

    /// A horizontal rule
    Rule,

    /// HTML content (preserved as-is)
    Html(String),
}

/// A list item, which may contain multiple blocks
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Whether this is a task list item
    pub task_list: Option<bool>,

    /// The content of the list item (can be multiple blocks)
    pub content: Vec<MarkdownBlock>,
}

impl ListItem {
    /// Create a new list item
    pub fn new() -> Self {
        Self {
            task_list: None,
            content: Vec::new(),
        }
    }

    /// Create a new list item with a single paragraph
    pub fn with_paragraph(runs: Vec<TextRun>) -> Self {
        Self {
            task_list: None,
            content: vec![MarkdownBlock::Paragraph(runs)],
        }
    }
}

impl Default for ListItem {
    fn default() -> Self {
        Self::new()
    }
}
