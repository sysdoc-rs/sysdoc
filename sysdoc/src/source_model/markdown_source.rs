//! Markdown source file representation

use super::blocks::MarkdownBlock;
use super::error::SourceModelError;
use super::section_number::SectionNumber;
use std::path::{Path, PathBuf};

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
    ///
    /// Parses the raw markdown content and populates the sections field
    /// with structured markdown sections delimited by headings.
    ///
    /// # Parameters
    /// * `document_root` - Root directory of the document for resolving relative image paths
    ///
    /// # Returns
    /// * `Ok(())` - Successfully parsed markdown
    /// * `Err(SourceModelError)` - Parse/validation error (e.g., invalid heading structure)
    ///
    /// # Validation Rules
    /// * Source markdown must contain at least one heading
    /// * The first heading must be level 1 (h1)
    /// * Only the first heading may be level 1 (all subsequent headings must be h2+)
    pub fn parse(&mut self, document_root: &Path) -> Result<(), SourceModelError> {
        let sections = super::parser::MarkdownParser::parse(
            &self.raw_content,
            document_root,
            &self.section_number,
        )?;

        // CSV tables are now embedded as CsvTable blocks within sections
        self.sections = sections;
        Ok(())
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

    /// Section number combining file section number + heading level increments
    /// For example, if the file is "01.02_foo.md" and this is the second h2 heading,
    /// the section_number would be [1, 2, 2] (01.02 from file + 2 from being the 2nd heading)
    pub section_number: SectionNumber,

    /// Parsed markdown content as structured blocks
    /// CSV tables are embedded as CsvTable blocks within this content
    pub content: Vec<MarkdownBlock>,
}
