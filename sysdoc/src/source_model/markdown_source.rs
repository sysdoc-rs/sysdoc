//! Markdown source file representation

use super::blocks::MarkdownBlock;
use super::section_number::SectionNumber;
use std::path::PathBuf;

/// A single markdown source file with its parsed content
#[derive(Debug)]
pub struct MarkdownSource {
    /// Path to the source file (relative to document root)
    pub path: PathBuf,

    /// Absolute path to the source file
    pub absolute_path: PathBuf,

    /// Section number parsed from filename (e.g., [1, 1] from "01.01_purpose.md")
    pub section_number: SectionNumber,

    /// Title derived from filename (e.g., "Purpose" from "01.01_purpose.md")
    pub title: String,

    /// Raw markdown content
    pub raw_content: String,

    /// Parsed sections (split by headings)
    pub sections: Vec<MarkdownSection>,
}

impl MarkdownSource {
    /// Parse the markdown content into sections
    pub fn parse(&mut self) {
        let (sections, _table_refs) = super::parser::MarkdownParser::parse(&self.raw_content);

        // The parser already integrates table refs into the sections
        self.sections = sections;
    }
}

/// A section within a markdown file (delimited by headings)
#[derive(Debug, Clone)]
pub struct MarkdownSection {
    /// Heading level (1 = h1, 2 = h2, etc.)
    /// The level indicates the nesting depth of the section within the source file from which it was parsed.
    pub heading_level: usize,

    /// Text content of the heading (as formatted text runs)
    pub heading_text: String,

    /// Parsed markdown content as structured blocks
    pub content: Vec<MarkdownBlock>,

    /// CSV tables referenced in this section
    pub table_refs: Vec<PathBuf>,
}
