//! Typst-based PDF export
//!
//! This module exports a UnifiedDocument to a PDF file using the Typst typesetting system.
//! It provides better typography and native SVG support compared to genpdf.

use crate::source_model::{ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

// Embedded Liberation Sans fonts (proportional - for body text)
const FONT_REGULAR: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Bold.ttf");
const FONT_ITALIC: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Italic.ttf");
const FONT_BOLD_ITALIC: &[u8] =
    include_bytes!("../../external/fonts/LiberationSans-BoldItalic.ttf");

// Embedded Liberation Mono fonts (monospace - for code blocks)
const FONT_MONO_REGULAR: &[u8] = include_bytes!("../../external/fonts/LiberationMono-Regular.ttf");
const FONT_MONO_BOLD: &[u8] = include_bytes!("../../external/fonts/LiberationMono-Bold.ttf");
const FONT_MONO_ITALIC: &[u8] = include_bytes!("../../external/fonts/LiberationMono-Italic.ttf");
const FONT_MONO_BOLD_ITALIC: &[u8] =
    include_bytes!("../../external/fonts/LiberationMono-BoldItalic.ttf");

/// Typst export errors
#[derive(Error, Debug)]
pub enum TypstExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Typst compilation failed: {0}")]
    CompilationError(String),

    #[error("Image error: {0}")]
    ImageError(String),

    #[error("Font loading error: {0}")]
    FontError(String),
}

/// Type alias for file cache data (files by ID and path-to-ID mapping)
type FileCache = (HashMap<FileId, Bytes>, HashMap<PathBuf, FileId>);

/// Static library instance (created once, reused)
static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();

/// Get or create the standard library
fn get_library() -> &'static LazyHash<Library> {
    LIBRARY.get_or_init(|| LazyHash::new(Library::builder().build()))
}

/// The World implementation for sysdoc
///
/// Provides the Typst compiler with access to:
/// - The generated Typst source code
/// - Embedded fonts
/// - Image files referenced in the document
struct SysdocWorld {
    /// The main Typst source file
    main_source: Source,
    /// Font book containing metadata about available fonts
    font_book: LazyHash<FontBook>,
    /// Loaded fonts
    fonts: Vec<Font>,
    /// File cache for images and other assets
    files: HashMap<FileId, Bytes>,
    /// Mapping from paths to FileIds
    path_to_id: HashMap<PathBuf, FileId>,
}

impl SysdocWorld {
    /// Create a new SysdocWorld with the given Typst source and document
    fn new(typst_source: String, doc: &UnifiedDocument) -> Result<Self, TypstExportError> {
        // Create the main source file
        let main_id = FileId::new(None, typst::syntax::VirtualPath::new("main.typ"));
        let main_source = Source::new(main_id, typst_source);

        // Load embedded fonts
        let (font_book, fonts) = Self::load_fonts()?;

        // Load image files into the file cache
        let (files, path_to_id) = Self::load_files(doc)?;

        Ok(Self {
            main_source,
            font_book: LazyHash::new(font_book),
            fonts,
            files,
            path_to_id,
        })
    }

    /// Load embedded fonts
    fn load_fonts() -> Result<(FontBook, Vec<Font>), TypstExportError> {
        let mut font_book = FontBook::new();
        let mut fonts = Vec::new();

        // Proportional fonts (Liberation Sans) for body text
        let sans_fonts = [
            ("Liberation Sans", FONT_REGULAR),
            ("Liberation Sans Bold", FONT_BOLD),
            ("Liberation Sans Italic", FONT_ITALIC),
            ("Liberation Sans Bold Italic", FONT_BOLD_ITALIC),
        ];

        // Monospace fonts (Liberation Mono) for code blocks
        let mono_fonts = [
            ("Liberation Mono", FONT_MONO_REGULAR),
            ("Liberation Mono Bold", FONT_MONO_BOLD),
            ("Liberation Mono Italic", FONT_MONO_ITALIC),
            ("Liberation Mono Bold Italic", FONT_MONO_BOLD_ITALIC),
        ];

        for (_name, data) in sans_fonts.iter().chain(mono_fonts.iter()) {
            let bytes = Bytes::new(data.to_vec());
            for font in Font::iter(bytes) {
                font_book.push(font.info().clone());
                fonts.push(font);
            }
        }

        if fonts.is_empty() {
            return Err(TypstExportError::FontError(
                "No fonts could be loaded".to_string(),
            ));
        }

        Ok((font_book, fonts))
    }

    /// Load image files referenced in the document
    fn load_files(doc: &UnifiedDocument) -> Result<FileCache, TypstExportError> {
        let mut files = HashMap::new();
        let mut path_to_id = HashMap::new();

        // Collect all image paths from the document
        for section in &doc.sections {
            Self::collect_image_files(&section.content, &mut files, &mut path_to_id)?;
        }

        Ok((files, path_to_id))
    }

    /// Recursively collect image files from blocks
    fn collect_image_files(
        blocks: &[MarkdownBlock],
        files: &mut HashMap<FileId, Bytes>,
        path_to_id: &mut HashMap<PathBuf, FileId>,
    ) -> Result<(), TypstExportError> {
        for block in blocks {
            Self::collect_image_from_block(block, files, path_to_id)?;
        }
        Ok(())
    }

    /// Collect image from a single block (helper to reduce nesting)
    fn collect_image_from_block(
        block: &MarkdownBlock,
        files: &mut HashMap<FileId, Bytes>,
        path_to_id: &mut HashMap<PathBuf, FileId>,
    ) -> Result<(), TypstExportError> {
        match block {
            MarkdownBlock::Image {
                absolute_path,
                exists: true,
                ..
            } => {
                if let Ok(data) = std::fs::read(absolute_path) {
                    let file_id = FileId::new(None, typst::syntax::VirtualPath::new(absolute_path));
                    files.insert(file_id, Bytes::new(data));
                    path_to_id.insert(absolute_path.clone(), file_id);
                }
            }
            MarkdownBlock::BlockQuote(inner) => {
                Self::collect_image_files(inner, files, path_to_id)?;
            }
            MarkdownBlock::List { items, .. } => {
                for item in items {
                    Self::collect_image_files(&item.content, files, path_to_id)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl typst::World for SysdocWorld {
    fn library(&self) -> &LazyHash<Library> {
        get_library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_book
    }

    fn main(&self) -> FileId {
        self.main_source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_source.id() {
            Ok(self.main_source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // Return current date
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).ok()?;
        let secs = duration.as_secs() as i64;

        // Simple date calculation (not timezone aware)
        let days = secs / 86400;
        let year = 1970 + (days / 365) as i32; // Approximate
        let month = ((days % 365) / 30 + 1) as u8;
        let day = ((days % 365) % 30 + 1) as u8;

        Datetime::from_ymd(year, month, day)
    }
}

/// Format error location from a Typst source diagnostic
fn format_error_location(error: &typst::diag::SourceDiagnostic, world: &SysdocWorld) -> String {
    let Some(id) = error.span.id() else {
        return "unknown".to_string();
    };

    let Ok(source) = World::source(world, id) else {
        return id.vpath().as_rootless_path().display().to_string();
    };

    let Some(range) = source.range(error.span) else {
        return id.vpath().as_rootless_path().display().to_string();
    };

    let line = source.byte_to_line(range.start).unwrap_or(0) + 1;
    let col = source.byte_to_column(range.start).unwrap_or(0) + 1;
    format!(
        "{}:{}:{}",
        id.vpath().as_rootless_path().display(),
        line,
        col
    )
}

/// Export a unified document to PDF using Typst
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `output_path` - Path where the PDF file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to PDF
/// * `Err(TypstExportError)` - Error during export
pub fn to_pdf(doc: &UnifiedDocument, output_path: &Path) -> Result<(), TypstExportError> {
    // Step 1: Generate Typst markup from the document
    let typst_source = generate_typst_markup(doc);

    // Step 2: Create the World with fonts and file access
    let world = SysdocWorld::new(typst_source, doc)?;

    // Step 3: Compile the Typst document
    let result = typst::compile(&world);

    let document = result.output.map_err(|errors| {
        let error_msgs: Vec<String> = errors
            .iter()
            .map(|e| {
                let location = format_error_location(e, &world);
                format!("{}: {}", location, e.message)
            })
            .collect();
        TypstExportError::CompilationError(error_msgs.join("\n"))
    })?;

    // Step 4: Export to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|e| TypstExportError::CompilationError(format!("PDF export failed: {:?}", e)))?;

    // Step 5: Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Step 6: Write to file
    std::fs::write(output_path, pdf_bytes)?;

    Ok(())
}

/// Generate Typst preamble with header/footer definitions
///
/// If protection_mark is set, it appears centered in red on all pages.
fn generate_preamble(doc: &UnifiedDocument) -> String {
    let document_id = escape_typst_string(&doc.metadata.document_id);
    let version = escape_typst_string(doc.metadata.version.as_deref().unwrap_or("Draft"));

    let mut preamble = String::new();

    // Document metadata
    preamble.push_str(&format!(
        "#set document(title: \"{}\", author: \"{}\")\n",
        escape_typst(&doc.metadata.title),
        escape_typst(&doc.metadata.owner.name),
    ));

    if let Some(ref mark) = doc.metadata.protection_mark {
        let escaped_mark = escape_typst_string(mark);

        // Define protection mark variable and custom header/footer functions
        preamble.push_str(&format!(
            r#"#let protection-mark = "{}"
#let doc-id = "{}"
#let doc-version = "{}"

#let make-header() = {{
  align(center)[#text(size: 12pt, fill: red, weight: "bold")[#protection-mark]]
  if counter(page).get().first() > 1 {{
    v(-1em)
    text(size: 10pt)[#doc-id]
    h(1fr)
    text(size: 10pt)[#doc-version]
  }}
}}

#let make-footer() = {{
  grid(
    columns: (1fr, auto, 1fr),
    align: (left, center, right),
    [],
    text(size: 12pt, fill: red, weight: "bold")[#protection-mark],
    text(size: 10pt)[Page #counter(page).display() of #counter(page).final().first()]
  )
}}

#set page(paper: "a4", margin: 2cm, header: context {{ make-header() }}, footer: context {{ make-footer() }})
"#,
            escaped_mark, document_id, version
        ));
    } else {
        // No protection mark - simpler header/footer
        preamble.push_str(&format!(
            r#"#let doc-id = "{}"
#let doc-version = "{}"

#set page(paper: "a4", margin: 2cm, header: context {{
  if counter(page).get().first() > 1 {{
    text(size: 10pt)[#doc-id]
    h(1fr)
    text(size: 10pt)[#doc-version]
  }}
}}, footer: context {{
  h(1fr)
  text(size: 10pt)[Page #counter(page).display() of #counter(page).final().first()]
}})
"#,
            document_id, version
        ));
    }

    // Common styling
    preamble.push_str(
        r#"#set text(font: "Liberation Sans", size: 11pt)
#set heading(numbering: none)

// Code styling: monospace font for all code
#show raw: set text(font: "Liberation Mono", size: 9pt)
// Code block styling: off-white background with tighter line spacing
#show raw.where(block: true): it => block(
  fill: luma(245),
  inset: 8pt,
  radius: 4pt,
  width: 100%,
  breakable: false,
)[#set par(leading: 0.5em); #it]

// Keep headings with following content (avoid orphaned headings at page bottom)
#show heading: it => block(above: 1.4em, below: 0.6em, sticky: true)[#it]

"#,
    );

    preamble
}

/// Generate Typst markup from a UnifiedDocument
fn generate_typst_markup(doc: &UnifiedDocument) -> String {
    let mut output = String::new();

    // Generate preamble with document setup and header/footer
    output.push_str(&generate_preamble(doc));

    // Title page
    output.push_str(&generate_title_page(doc));

    // Page break before TOC
    output.push_str("#pagebreak()\n\n");

    // Table of contents
    output.push_str(
        r#"#outline(title: "Table of Contents", depth: 3)
#pagebreak()

"#,
    );

    // Content sections
    for section in &doc.sections {
        output.push_str(&generate_section(section));
    }

    output
}

/// Generate Typst markup for the title page
fn generate_title_page(doc: &UnifiedDocument) -> String {
    let mut output = String::new();

    output.push_str("#v(4em)\n");
    output.push_str(&format!(
        "#align(center)[#text(size: 24pt, weight: \"bold\")[{}]]\n",
        escape_typst(&doc.metadata.title)
    ));

    if let Some(subtitle) = &doc.metadata.subtitle {
        output.push_str("#v(1em)\n");
        output.push_str(&format!(
            "#align(center)[#text(size: 16pt)[{}]]\n",
            escape_typst(subtitle)
        ));
    }

    output.push_str("#v(3em)\n\n");

    // Metadata table
    output.push_str(&format!(
        "*Document ID:* {}\n\n",
        escape_typst(&doc.metadata.document_id)
    ));
    output.push_str(&format!(
        "*Type:* {}\n\n",
        escape_typst(&doc.metadata.doc_type)
    ));
    output.push_str(&format!(
        "*Standard:* {}\n\n",
        escape_typst(&doc.metadata.standard)
    ));
    output.push_str(&format!(
        "*Owner:* {}\n\n",
        escape_typst(&doc.metadata.owner.name)
    ));
    output.push_str(&format!(
        "*Approver:* {}\n\n",
        escape_typst(&doc.metadata.approver.name)
    ));

    if let Some(version) = &doc.metadata.version {
        output.push_str(&format!("*Version:* {}\n\n", escape_typst(version)));
    }

    if let Some(created) = &doc.metadata.created {
        output.push_str(&format!("*Created:* {}\n\n", escape_typst(created)));
    }

    if let Some(modified) = &doc.metadata.modified {
        output.push_str(&format!("*Last Modified:* {}\n\n", escape_typst(modified)));
    }

    if let Some(description) = &doc.metadata.description {
        output.push_str("#v(1.5em)\n");
        output.push_str("*Description:*\n\n");
        output.push_str(&format!("{}\n\n", escape_typst(description)));
    }

    output
}

/// Generate Typst markup for a section
fn generate_section(section: &MarkdownSection) -> String {
    let mut output = String::new();

    // Section heading with number
    let heading_prefix = "=".repeat(section.heading_level);
    output.push_str(&format!(
        "{} {} {}\n\n",
        heading_prefix,
        section.section_number,
        escape_typst(&section.heading_text)
    ));

    // Section content
    for block in &section.content {
        output.push_str(&generate_block(block));
    }

    output
}

/// Generate Typst markup for a block
fn generate_block(block: &MarkdownBlock) -> String {
    match block {
        MarkdownBlock::Paragraph(runs) => {
            format!("{}\n\n", runs_to_typst(runs))
        }

        MarkdownBlock::Heading { level, runs } => {
            let prefix = "=".repeat(*level);
            format!("{} {}\n\n", prefix, runs_to_typst(runs))
        }

        MarkdownBlock::CodeBlock { code, language, .. } => {
            let lang = language.as_deref().unwrap_or("");
            format!("```{}\n{}\n```\n\n", lang, code)
        }

        MarkdownBlock::IncludedCodeBlock {
            path,
            language,
            content,
            exists,
            ..
        } => {
            if !exists {
                format!(
                    "_[Included file not found: {}]_\n\n",
                    escape_typst(&path.display().to_string())
                )
            } else if let Some(code) = content {
                let lang = language.as_deref().unwrap_or("");
                format!("```{}\n{}\n```\n\n", lang, code)
            } else {
                format!(
                    "_[Included file could not be read: {}]_\n\n",
                    escape_typst(&path.display().to_string())
                )
            }
        }

        MarkdownBlock::BlockQuote(blocks) => {
            let mut content = String::new();
            for block in blocks {
                content.push_str(&generate_block(block));
            }
            format!("#quote[{}]\n\n", content.trim())
        }

        MarkdownBlock::List { start, items } => {
            let mut output = String::new();
            for (idx, item) in items.iter().enumerate() {
                let prefix = if let Some(start_num) = start {
                    format!("{}. ", start_num + idx as u64)
                } else {
                    "- ".to_string()
                };
                output.push_str(&generate_list_item(item, &prefix));
            }
            output.push('\n');
            output
        }

        MarkdownBlock::InlineTable {
            headers,
            rows,
            alignments,
        } => generate_table(headers, rows, alignments),

        MarkdownBlock::CsvTable {
            path, exists, data, ..
        } => {
            if !exists {
                format!(
                    "_[CSV file not found: {}]_\n\n",
                    escape_typst(&path.display().to_string())
                )
            } else if let Some(csv_data) = data {
                generate_csv_table(csv_data)
            } else {
                "_[CSV table - data not loaded]_\n\n".to_string()
            }
        }

        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            exists,
            path,
            ..
        } => {
            if !exists {
                format!(
                    "_[Image file not found: {}]_\n\n",
                    escape_typst(&path.display().to_string())
                )
            } else {
                let mut output = format!(
                    "#figure(\n  image(\"{}\", width: 80%),\n",
                    absolute_path.display().to_string().replace('\\', "/")
                );
                if !alt_text.is_empty() {
                    output.push_str(&format!("  caption: [{}],\n", escape_typst(alt_text)));
                }
                output.push_str(")\n\n");
                output
            }
        }

        MarkdownBlock::Rule => "#line(length: 100%)\n\n".to_string(),

        MarkdownBlock::Html(_) => {
            // Skip HTML blocks in Typst output
            String::new()
        }
    }
}

/// Generate Typst markup for a list item
fn generate_list_item(item: &ListItem, prefix: &str) -> String {
    let mut output = String::new();

    for (idx, block) in item.content.iter().enumerate() {
        if idx == 0 {
            // First block gets the bullet/number prefix
            if let MarkdownBlock::Paragraph(ref runs) = *block {
                let task_marker = match item.task_list {
                    Some(true) => "[x] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                output.push_str(&format!(
                    "{}{}{}\n",
                    prefix,
                    task_marker,
                    runs_to_typst(runs)
                ));
            } else {
                output.push_str(prefix);
                output.push_str(&generate_block(block));
            }
        } else {
            // Subsequent blocks are indented
            output.push_str("  ");
            output.push_str(&generate_block(block));
        }
    }

    output
}

/// Generate Typst markup for an inline table
fn generate_table(
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
    _alignments: &[crate::source_model::Alignment],
) -> String {
    let num_cols = headers.len();
    if num_cols == 0 {
        return String::new();
    }

    let mut output = format!("#table(\n  columns: {},\n", num_cols);

    // Header row
    for cell in headers {
        output.push_str(&format!("  [*{}*],\n", runs_to_typst(cell)));
    }

    // Data rows
    for row in rows {
        for cell in row {
            output.push_str(&format!("  [{}],\n", runs_to_typst(cell)));
        }
        // Fill missing cells
        for _ in row.len()..num_cols {
            output.push_str("  [],\n");
        }
    }

    output.push_str(")\n\n");
    output
}

/// Generate Typst markup for a CSV table
fn generate_csv_table(data: &[Vec<String>]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let num_cols = data.first().map(|r| r.len()).unwrap_or(0);
    if num_cols == 0 {
        return String::new();
    }

    let mut output = format!("#table(\n  columns: {},\n", num_cols);

    // First row is header (bold)
    if let Some(header) = data.first() {
        for cell in header {
            output.push_str(&format!("  [*{}*],\n", escape_typst(cell)));
        }
    }

    // Data rows
    for row in data.iter().skip(1) {
        for cell in row {
            output.push_str(&format!("  [{}],\n", escape_typst(cell)));
        }
        // Fill missing cells
        for _ in row.len()..num_cols {
            output.push_str("  [],\n");
        }
    }

    output.push_str(")\n\n");
    output
}

/// Convert text runs to Typst markup
fn runs_to_typst(runs: &[TextRun]) -> String {
    let mut output = String::new();

    for run in runs {
        let mut text = escape_typst(&run.text);

        // Apply formatting
        if run.code {
            text = format!("`{}`", run.text); // Don't escape code
        } else {
            if run.bold && run.italic {
                text = format!("_*{}*_", text);
            } else if run.bold {
                text = format!("*{}*", text);
            } else if run.italic {
                text = format!("_{}_", text);
            }

            if run.strikethrough {
                text = format!("#strike[{}]", text);
            }

            if run.superscript {
                text = format!("#super[{}]", text);
            }

            if run.subscript {
                text = format!("#sub[{}]", text);
            }

            if let Some(url) = &run.link_url {
                text = format!("#link(\"{}\")[{}]", escape_typst(url), text);
            }
        }

        output.push_str(&text);
    }

    output
}

/// Escape special characters for Typst
fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('"', "\\\"")
}

/// Escape string for use inside Typst string literals (only quotes and backslashes)
fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_typst() {
        assert_eq!(escape_typst("hello"), "hello");
        assert_eq!(escape_typst("hello*world"), "hello\\*world");
        assert_eq!(escape_typst("#test"), "\\#test");
    }

    #[test]
    fn test_escape_typst_string() {
        assert_eq!(escape_typst_string("hello"), "hello");
        assert_eq!(escape_typst_string("DI-IPSC-81435B"), "DI-IPSC-81435B");
        assert_eq!(
            escape_typst_string("test_with_underscores"),
            "test_with_underscores"
        );
        assert_eq!(escape_typst_string("quote\"test"), "quote\\\"test");
        assert_eq!(escape_typst_string("back\\slash"), "back\\\\slash");
        assert_eq!(
            escape_typst_string("special*chars#are@not[escaped]"),
            "special*chars#are@not[escaped]"
        );
    }

    #[test]
    fn test_runs_to_typst_plain() {
        let runs = vec![TextRun {
            text: "Hello".to_string(),
            bold: false,
            italic: false,
            code: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            link_url: None,
            link_title: None,
        }];
        assert_eq!(runs_to_typst(&runs), "Hello");
    }

    #[test]
    fn test_runs_to_typst_bold() {
        let runs = vec![TextRun {
            text: "Bold".to_string(),
            bold: true,
            italic: false,
            code: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            link_url: None,
            link_title: None,
        }];
        assert_eq!(runs_to_typst(&runs), "*Bold*");
    }
}
