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
    Heading {
        /// Heading level (1 = h1, 2 = h2, etc.)
        level: usize,
        /// Formatted text runs comprising the heading content
        runs: Vec<TextRun>,
    },

    /// A paragraph of formatted text
    ///
    /// Contains a vector of text runs with various formatting applied
    Paragraph(Vec<TextRun>),

    /// An image reference with metadata
    Image {
        /// Path to the image file (relative to document root, as written in markdown)
        path: PathBuf,
        /// Absolute path to the image file
        absolute_path: PathBuf,
        /// Alternative text for the image (for accessibility)
        alt_text: String,
        /// Optional title text displayed on hover
        title: String,
        /// Image format (png, jpg, svg, etc.)
        format: super::image::ImageFormat,
        /// Whether the image file exists on disk
        exists: bool,
    },

    /// A code block
    CodeBlock {
        /// Programming language for syntax highlighting (e.g., "rust", "python")
        language: Option<String>,
        /// Raw code content
        code: String,
        /// Whether this is a fenced code block (```) or indented code block
        fenced: bool,
    },

    /// A block quote containing other blocks
    ///
    /// Nested blocks that are quoted (typically rendered with left border/indentation)
    BlockQuote(Vec<MarkdownBlock>),

    /// An ordered or unordered list
    List {
        /// Starting number for ordered lists (e.g., Some(1)), None for unordered lists
        start: Option<u64>,
        /// List items, each containing nested blocks
        items: Vec<ListItem>,
    },

    /// An inline markdown table
    ///
    /// Represents tables defined using markdown pipe syntax (| col1 | col2 |)
    InlineTable {
        /// Column alignment specifications (left, center, right, none)
        alignments: Vec<Alignment>,
        /// Header row cells, each cell containing formatted text runs
        headers: Vec<Vec<TextRun>>,
        /// Data rows, where each row contains cells, and each cell contains text runs
        rows: Vec<Vec<Vec<TextRun>>>,
    },

    /// A CSV table reference with loaded data
    ///
    /// Represents a table loaded from an external CSV file
    CsvTable {
        /// Path to the CSV file (relative to document root, as written in markdown)
        path: PathBuf,
        /// Absolute path to the CSV file
        absolute_path: PathBuf,
        /// Whether the CSV file exists on disk
        exists: bool,
        /// Parsed CSV data (headers + rows) if loaded successfully
        data: Option<Vec<Vec<String>>>,
    },

    /// A horizontal rule (thematic break)
    ///
    /// Typically rendered as a horizontal line (created with ---, ***, or ___)
    Rule,

    /// HTML content (preserved as-is)
    ///
    /// Raw HTML that should be passed through without modification
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
    ///
    /// # Returns
    /// * `ListItem` - A new empty list item with no content
    pub fn new() -> Self {
        Self {
            task_list: None,
            content: Vec::new(),
        }
    }

    /// Create a new list item with a single paragraph
    ///
    /// # Parameters
    /// * `runs` - Text runs to include in the paragraph
    ///
    /// # Returns
    /// * `ListItem` - A new list item containing a single paragraph block
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
