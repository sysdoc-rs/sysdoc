//! Source model for the parsing stage
//!
//! This module defines the structures used during Stage 1 (Parsing)
//! where markdown files, images, and CSV files are loaded and validated.

use crate::document_config::DocumentConfig;
use pulldown_cmark::Event;
use std::path::PathBuf;

/// Collection of all source files discovered and parsed
#[derive(Debug)]
pub struct SourceModel {
    /// Root directory of the document
    pub root: PathBuf,

    /// Document configuration from sysdoc.toml
    pub config: DocumentConfig,

    /// All markdown source files, ordered by discovery (not sorted yet)
    pub markdown_files: Vec<MarkdownSource>,

    /// All image files referenced in the markdown
    pub image_files: Vec<ImageSource>,

    /// All CSV table files referenced in the markdown
    pub table_files: Vec<TableSource>,
}

impl SourceModel {
    /// Create a new empty source model
    pub fn new(root: PathBuf, config: DocumentConfig) -> Self {
        Self {
            root,
            config,
            markdown_files: Vec::new(),
            image_files: Vec::new(),
            table_files: Vec::new(),
        }
    }

    /// Validate that all referenced resources exist
    pub fn validate(&self) -> Result<(), ValidationError> {
        let image_errors = self.validate_image_references();
        let table_errors = self.validate_table_references();

        let errors: Vec<ValidationError> = image_errors.into_iter().chain(table_errors).collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }

    /// Validate all image references
    fn validate_image_references(&self) -> Vec<ValidationError> {
        self.markdown_files
            .iter()
            .flat_map(|md_file| {
                md_file
                    .sections
                    .iter()
                    .flat_map(|section| self.validate_section_images(md_file, section))
            })
            .collect()
    }

    /// Validate image references in a single section
    fn validate_section_images(
        &self,
        md_file: &MarkdownSource,
        section: &MarkdownSection,
    ) -> Vec<ValidationError> {
        section
            .image_refs
            .iter()
            .filter(|img_ref| !self.image_files.iter().any(|img| img.path == img_ref.path))
            .map(|img_ref| ValidationError::MissingImage {
                referenced_in: md_file.path.clone(),
                image_path: img_ref.path.clone(),
            })
            .collect()
    }

    /// Validate all table references
    fn validate_table_references(&self) -> Vec<ValidationError> {
        self.markdown_files
            .iter()
            .flat_map(|md_file| {
                md_file
                    .sections
                    .iter()
                    .flat_map(|section| self.validate_section_tables(md_file, section))
            })
            .collect()
    }

    /// Validate table references in a single section
    fn validate_section_tables(
        &self,
        md_file: &MarkdownSource,
        section: &MarkdownSection,
    ) -> Vec<ValidationError> {
        section
            .table_refs
            .iter()
            .filter(|table_ref| !self.table_files.iter().any(|tbl| &tbl.path == *table_ref))
            .map(|table_ref| ValidationError::MissingTable {
                referenced_in: md_file.path.clone(),
                table_path: table_ref.clone(),
            })
            .collect()
    }
}

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
        let mut sections = Vec::new();
        let mut current_section: Option<MarkdownSection> = None;

        let parser = pulldown_cmark::Parser::new(&self.raw_content);

        for event in parser {
            current_section = Self::process_event(event, current_section, &mut sections);
        }

        // Don't forget the last section
        if let Some(section) = current_section {
            sections.push(section);
        }

        self.sections = sections;
    }

    /// Process a single markdown event and update section state
    fn process_event(
        event: Event<'_>,
        current_section: Option<MarkdownSection>,
        sections: &mut Vec<MarkdownSection>,
    ) -> Option<MarkdownSection> {
        // Extract data from event before consuming it
        match event {
            Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                Self::start_new_section(level as usize, current_section, sections)
            }
            Event::Text(ref text) => {
                let text_str = text.to_string();
                let event_clone = event.clone();
                Self::process_text(&text_str, current_section, event_clone)
            }
            Event::Start(pulldown_cmark::Tag::Image { ref dest_url, .. }) => {
                let url = dest_url.to_string();
                let event_clone = event.clone();
                Self::process_image(&url, current_section, event_clone)
            }
            Event::Start(pulldown_cmark::Tag::Link { ref dest_url, .. }) => {
                let url = dest_url.to_string();
                let event_clone = event.clone();
                Self::process_link(&url, current_section, event_clone)
            }
            _ => Self::add_content_to_section(current_section, event),
        }
    }

    /// Start a new section and save the previous one
    fn start_new_section(
        level: usize,
        current_section: Option<MarkdownSection>,
        sections: &mut Vec<MarkdownSection>,
    ) -> Option<MarkdownSection> {
        if let Some(section) = current_section {
            sections.push(section);
        }

        Some(MarkdownSection {
            heading_level: level,
            heading_text: String::new(),
            content: Vec::new(),
            image_refs: Vec::new(),
            table_refs: Vec::new(),
        })
    }

    /// Process text event
    fn process_text(
        text: &str,
        mut current_section: Option<MarkdownSection>,
        event: Event<'_>,
    ) -> Option<MarkdownSection> {
        if let Some(ref mut section) = current_section {
            // If we're collecting heading text (no content yet)
            if section.content.is_empty() && section.heading_text.is_empty() {
                section.heading_text = text.to_string();
            }
            section.content.push(MarkdownContent::from_event(event));
        }
        current_section
    }

    /// Process image event
    fn process_image(
        dest_url: &str,
        mut current_section: Option<MarkdownSection>,
        event: Event<'_>,
    ) -> Option<MarkdownSection> {
        if let Some(ref mut section) = current_section {
            section.image_refs.push(ImageReference {
                path: PathBuf::from(dest_url),
                alt_text: String::new(),
            });
            section.content.push(MarkdownContent::from_event(event));
        }
        current_section
    }

    /// Process link event (check for CSV tables)
    fn process_link(
        dest_url: &str,
        mut current_section: Option<MarkdownSection>,
        event: Event<'_>,
    ) -> Option<MarkdownSection> {
        if let Some(ref mut section) = current_section {
            if dest_url.ends_with(".csv") {
                section.table_refs.push(PathBuf::from(dest_url));
            }
            section.content.push(MarkdownContent::from_event(event));
        }
        current_section
    }

    /// Add content to current section
    fn add_content_to_section(
        mut current_section: Option<MarkdownSection>,
        event: Event<'_>,
    ) -> Option<MarkdownSection> {
        if let Some(ref mut section) = current_section {
            section.content.push(MarkdownContent::from_event(event));
        }
        current_section
    }
}

/// A section within a markdown file (delimited by headings)
#[derive(Debug, Clone)]
pub struct MarkdownSection {
    /// Heading level (1 = h1, 2 = h2, etc.)
    pub heading_level: usize,

    /// Text content of the heading
    pub heading_text: String,

    /// Parsed markdown content as structured elements
    pub content: Vec<MarkdownContent>,

    /// Images referenced in this section
    pub image_refs: Vec<ImageReference>,

    /// CSV tables referenced in this section
    pub table_refs: Vec<PathBuf>,
}

/// Structured representation of markdown content
#[derive(Debug, Clone)]
pub enum MarkdownContent {
    /// Start of a tag/element
    Start(MarkdownTag),
    /// End of a tag/element
    End(MarkdownTagEnd),
    /// Plain text content
    Text(String),
    /// Code block text
    Code(String),
    /// HTML content (including comments)
    Html(String),
    /// Inline code
    InlineCode(String),
    /// Line break
    SoftBreak,
    /// Hard break
    HardBreak,
    /// Horizontal rule
    Rule,
    /// Footnote reference
    FootnoteReference(String),
    /// Task list marker
    TaskListMarker(bool),
}

impl MarkdownContent {
    /// Convert a pulldown_cmark Event to MarkdownContent
    pub fn from_event(event: Event<'_>) -> Self {
        match event {
            Event::Start(tag) => MarkdownContent::Start(MarkdownTag::from_tag(tag)),
            Event::End(tag_end) => MarkdownContent::End(MarkdownTagEnd::from_tag_end(tag_end)),
            Event::Text(text) => MarkdownContent::Text(text.to_string()),
            Event::Code(code) => MarkdownContent::Code(code.to_string()),
            Event::Html(html) => MarkdownContent::Html(html.to_string()),
            Event::SoftBreak => MarkdownContent::SoftBreak,
            Event::HardBreak => MarkdownContent::HardBreak,
            Event::Rule => MarkdownContent::Rule,
            Event::FootnoteReference(fr) => MarkdownContent::FootnoteReference(fr.to_string()),
            Event::TaskListMarker(checked) => MarkdownContent::TaskListMarker(checked),
            Event::InlineHtml(html) => MarkdownContent::Html(html.to_string()),
            Event::InlineMath(math) => MarkdownContent::Code(math.to_string()),
            Event::DisplayMath(math) => MarkdownContent::Code(math.to_string()),
        }
    }
}

/// Markdown tag types (opening tags)
#[derive(Debug, Clone)]
pub enum MarkdownTag {
    Paragraph,
    Heading {
        level: usize,
        id: Option<String>,
        classes: Vec<String>,
        attrs: Vec<(String, Option<String>)>,
    },
    BlockQuote,
    CodeBlock {
        kind: CodeBlockKind,
        lang: Option<String>,
    },
    List {
        start: Option<u64>,
    },
    Item,
    FootnoteDefinition(String),
    Table {
        alignments: Vec<Alignment>,
    },
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Superscript,
    Subscript,
    Link {
        dest_url: String,
        title: String,
        id: String,
    },
    Image {
        dest_url: String,
        title: String,
        id: String,
    },
}

impl MarkdownTag {
    fn from_tag(tag: pulldown_cmark::Tag<'_>) -> Self {
        match tag {
            pulldown_cmark::Tag::Paragraph => MarkdownTag::Paragraph,
            pulldown_cmark::Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => MarkdownTag::Heading {
                level: level as usize,
                id: id.map(|s| s.to_string()),
                classes: classes.iter().map(|s| s.to_string()).collect(),
                attrs: attrs
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.as_ref().map(|s| s.to_string())))
                    .collect(),
            },
            pulldown_cmark::Tag::BlockQuote(_) => MarkdownTag::BlockQuote,
            pulldown_cmark::Tag::CodeBlock(kind) => {
                let (cb_kind, lang) = match kind {
                    pulldown_cmark::CodeBlockKind::Indented => (CodeBlockKind::Indented, None),
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        (CodeBlockKind::Fenced, Some(lang.to_string()))
                    }
                };
                MarkdownTag::CodeBlock {
                    kind: cb_kind,
                    lang,
                }
            }
            pulldown_cmark::Tag::List(start) => MarkdownTag::List { start },
            pulldown_cmark::Tag::Item => MarkdownTag::Item,
            pulldown_cmark::Tag::FootnoteDefinition(label) => {
                MarkdownTag::FootnoteDefinition(label.to_string())
            }
            pulldown_cmark::Tag::Table(alignments) => {
                let aligns = alignments
                    .iter()
                    .map(|a| match a {
                        pulldown_cmark::Alignment::None => Alignment::None,
                        pulldown_cmark::Alignment::Left => Alignment::Left,
                        pulldown_cmark::Alignment::Center => Alignment::Center,
                        pulldown_cmark::Alignment::Right => Alignment::Right,
                    })
                    .collect();
                MarkdownTag::Table { alignments: aligns }
            }
            pulldown_cmark::Tag::TableHead => MarkdownTag::TableHead,
            pulldown_cmark::Tag::TableRow => MarkdownTag::TableRow,
            pulldown_cmark::Tag::TableCell => MarkdownTag::TableCell,
            pulldown_cmark::Tag::Emphasis => MarkdownTag::Emphasis,
            pulldown_cmark::Tag::Strong => MarkdownTag::Strong,
            pulldown_cmark::Tag::Strikethrough => MarkdownTag::Strikethrough,
            pulldown_cmark::Tag::Superscript => MarkdownTag::Superscript,
            pulldown_cmark::Tag::Subscript => MarkdownTag::Subscript,
            pulldown_cmark::Tag::Link {
                dest_url,
                title,
                id,
                link_type: _,
            } => MarkdownTag::Link {
                dest_url: dest_url.to_string(),
                title: title.to_string(),
                id: id.to_string(),
            },
            pulldown_cmark::Tag::Image {
                dest_url,
                title,
                id,
                link_type: _,
            } => MarkdownTag::Image {
                dest_url: dest_url.to_string(),
                title: title.to_string(),
                id: id.to_string(),
            },
            pulldown_cmark::Tag::HtmlBlock => MarkdownTag::Paragraph, // Treat as paragraph for now
            pulldown_cmark::Tag::DefinitionList => MarkdownTag::List { start: None },
            pulldown_cmark::Tag::DefinitionListTitle => MarkdownTag::Item,
            pulldown_cmark::Tag::DefinitionListDefinition => MarkdownTag::Item,
            pulldown_cmark::Tag::MetadataBlock(_) => MarkdownTag::CodeBlock {
                kind: CodeBlockKind::Fenced,
                lang: Some("metadata".to_string()),
            },
        }
    }
}

/// Markdown tag end types (closing tags)
#[derive(Debug, Clone)]
pub enum MarkdownTagEnd {
    Paragraph,
    Heading,
    BlockQuote,
    CodeBlock,
    List,
    Item,
    FootnoteDefinition,
    Table,
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Superscript,
    Subscript,
    Link,
    Image,
}

impl MarkdownTagEnd {
    fn from_tag_end(tag_end: pulldown_cmark::TagEnd) -> Self {
        match tag_end {
            pulldown_cmark::TagEnd::Paragraph => MarkdownTagEnd::Paragraph,
            pulldown_cmark::TagEnd::Heading(_) => MarkdownTagEnd::Heading,
            pulldown_cmark::TagEnd::BlockQuote(_) => MarkdownTagEnd::BlockQuote,
            pulldown_cmark::TagEnd::CodeBlock => MarkdownTagEnd::CodeBlock,
            pulldown_cmark::TagEnd::List(_) => MarkdownTagEnd::List,
            pulldown_cmark::TagEnd::Item => MarkdownTagEnd::Item,
            pulldown_cmark::TagEnd::FootnoteDefinition => MarkdownTagEnd::FootnoteDefinition,
            pulldown_cmark::TagEnd::Table => MarkdownTagEnd::Table,
            pulldown_cmark::TagEnd::TableHead => MarkdownTagEnd::TableHead,
            pulldown_cmark::TagEnd::TableRow => MarkdownTagEnd::TableRow,
            pulldown_cmark::TagEnd::TableCell => MarkdownTagEnd::TableCell,
            pulldown_cmark::TagEnd::Emphasis => MarkdownTagEnd::Emphasis,
            pulldown_cmark::TagEnd::Strong => MarkdownTagEnd::Strong,
            pulldown_cmark::TagEnd::Strikethrough => MarkdownTagEnd::Strikethrough,
            pulldown_cmark::TagEnd::Superscript => MarkdownTagEnd::Superscript,
            pulldown_cmark::TagEnd::Subscript => MarkdownTagEnd::Subscript,
            pulldown_cmark::TagEnd::Link => MarkdownTagEnd::Link,
            pulldown_cmark::TagEnd::Image => MarkdownTagEnd::Image,
            pulldown_cmark::TagEnd::HtmlBlock => MarkdownTagEnd::Paragraph,
            pulldown_cmark::TagEnd::DefinitionList => MarkdownTagEnd::List,
            pulldown_cmark::TagEnd::DefinitionListTitle => MarkdownTagEnd::Item,
            pulldown_cmark::TagEnd::DefinitionListDefinition => MarkdownTagEnd::Item,
            pulldown_cmark::TagEnd::MetadataBlock(_) => MarkdownTagEnd::CodeBlock,
        }
    }
}

/// Code block types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockKind {
    Indented,
    Fenced,
}

/// Table cell alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

/// Reference to an image file
#[derive(Debug, Clone)]
pub struct ImageReference {
    /// Path to the image file (relative to document root)
    pub path: PathBuf,

    /// Alt text for the image
    pub alt_text: String,
}

/// An image source file
#[derive(Debug, Clone)]
pub struct ImageSource {
    /// Path to the image file (relative to document root)
    pub path: PathBuf,

    /// Absolute path to the image file
    pub absolute_path: PathBuf,

    /// Image format (png, jpg, svg, etc.)
    pub format: ImageFormat,

    /// Whether the image has been loaded into memory
    pub loaded: bool,

    /// Image data (if loaded)
    pub data: Option<Vec<u8>>,
}

impl ImageSource {
    /// Load the image data into memory
    pub fn load(&mut self) -> std::io::Result<()> {
        self.data = Some(std::fs::read(&self.absolute_path)?);
        self.loaded = true;
        Ok(())
    }
}

/// Image format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Svg,
    DrawIoSvg, // Special handling for .drawio.svg files
    Other,
}

impl ImageFormat {
    /// Determine format from file extension
    pub fn from_path(path: &std::path::Path) -> Self {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check for .drawio.svg first
        if path.to_string_lossy().ends_with(".drawio.svg") {
            return ImageFormat::DrawIoSvg;
        }

        match extension.as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "svg" => ImageFormat::Svg,
            _ => ImageFormat::Other,
        }
    }
}

/// A CSV table source file
#[derive(Debug, Clone)]
pub struct TableSource {
    /// Path to the CSV file (relative to document root)
    pub path: PathBuf,

    /// Absolute path to the CSV file
    pub absolute_path: PathBuf,

    /// Whether the table has been loaded into memory
    pub loaded: bool,

    /// Parsed CSV data (if loaded)
    pub data: Option<Vec<Vec<String>>>,
}

impl TableSource {
    /// Load and parse the CSV data
    pub fn load(&mut self) -> Result<(), csv::Error> {
        let mut reader = csv::Reader::from_path(&self.absolute_path)?;
        let mut data = Vec::new();

        for result in reader.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            data.push(row);
        }

        self.data = Some(data);
        self.loaded = true;
        Ok(())
    }
}

/// Section number representation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectionNumber {
    /// Number components (e.g., [1, 2, 3] for "01.02.03")
    pub parts: Vec<u32>,
}

impl SectionNumber {
    /// Parse section number from filename prefix
    /// Examples: "01.01" -> [1, 1], "02.03.01" -> [2, 3, 1]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Option<Vec<u32>> = s.split('.').map(|part| part.parse::<u32>().ok()).collect();

        parts.map(|parts| Self { parts })
    }

    /// Get the depth/nesting level (number of parts - 1)
    pub fn depth(&self) -> usize {
        self.parts.len().saturating_sub(1)
    }
}

impl std::fmt::Display for SectionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .parts
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", s)
    }
}

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
