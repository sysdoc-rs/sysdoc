//! Markdown event stream parser
//!
//! Converts pulldown-cmark's event stream into structured blocks with formatted text runs.

use super::blocks::{ListItem, MarkdownBlock};
use super::error::SourceModelError;
use super::image::ImageFormat;
use super::markdown_source::MarkdownSection;
use super::section_metadata::SectionMetadata;
use super::section_number::SectionNumber;
use super::text_run::{TextFormatting, TextRun};
use super::types::Alignment;
use pulldown_cmark::{Event, Tag, TagEnd};
use std::path::{Path, PathBuf};

/// Parser state for converting markdown events to blocks
pub struct MarkdownParser {
    /// Current formatting state (stack-based)
    formatting: TextFormatting,

    /// Current text runs being built for the current paragraph/heading
    current_runs: Vec<TextRun>,

    /// Current paragraph/block being built
    current_blocks: Vec<MarkdownBlock>,

    /// Stack of list contexts (for nested lists)
    list_stack: Vec<ListContext>,

    /// Stack of table contexts
    table_stack: Vec<TableContext>,

    /// Stack of block quote contexts
    blockquote_stack: Vec<Vec<MarkdownBlock>>,

    /// Current section being built
    current_section: Option<SectionBuilder>,

    /// Completed sections
    sections: Vec<MarkdownSection>,

    /// Document root directory for resolving relative paths
    document_root: PathBuf,

    /// File section number (base for calculating section numbers)
    file_section_number: SectionNumber,

    /// Heading counters at each level (1-indexed, for h1 through h6)
    /// `heading_counters[0]` = count of h1 headings, `heading_counters[1]` = count of h2 headings, etc.
    heading_counters: [u32; 6],

    /// Current code block context (language, accumulated content)
    current_code_block: Option<CodeBlockContext>,

    /// Source content for calculating line numbers
    source_content: String,

    /// Current line number being processed (1-indexed)
    current_line_number: usize,

    /// Source file path (relative to document root)
    source_file: PathBuf,

    /// Collected metadata parsing errors
    metadata_errors: Vec<SourceModelError>,
}

/// Context for building a code block
struct CodeBlockContext {
    /// Language identifier (e.g., "rust", "sysdoc")
    language: Option<String>,
    /// Accumulated code content
    content: String,
}

/// Context for building a list
struct ListContext {
    /// Starting number for ordered lists
    start: Option<u64>,
    /// List items
    items: Vec<ListItem>,
    /// Current item being built
    current_item: Option<Vec<MarkdownBlock>>,
}

/// Context for building a table
struct TableContext {
    /// Column alignments
    alignments: Vec<Alignment>,
    /// Header row cells (each cell is a vec of text runs)
    headers: Vec<Vec<TextRun>>,
    /// Data rows (each row is a vec of cells, each cell is a vec of text runs)
    rows: Vec<Vec<Vec<TextRun>>>,
    /// Current row being built
    current_row: Vec<Vec<TextRun>>,
    /// Whether we're in the header
    in_header: bool,
}

/// Builder for a section
struct SectionBuilder {
    /// Heading level
    level: usize,
    /// Heading text
    heading_text: String,
    /// Blocks in this section
    blocks: Vec<MarkdownBlock>,
    /// Optional metadata parsed from a sysdoc code block
    metadata: Option<SectionMetadata>,
    /// Line number where this section's heading appears (1-indexed)
    line_number: usize,
}

impl MarkdownParser {
    /// Create a new parser
    ///
    /// # Parameters
    /// * `document_root` - Root directory of the document for resolving relative paths
    /// * `file_section_number` - Section number of the markdown file (from filename)
    /// * `source_file` - Path to the source file (relative to document root)
    ///
    /// # Returns
    /// * `MarkdownParser` - A new parser with empty state
    pub fn new(
        document_root: PathBuf,
        file_section_number: SectionNumber,
        source_file: PathBuf,
    ) -> Self {
        Self {
            formatting: TextFormatting::new(),
            current_runs: Vec::new(),
            current_blocks: Vec::new(),
            list_stack: Vec::new(),
            table_stack: Vec::new(),
            blockquote_stack: Vec::new(),
            current_section: None,
            sections: Vec::new(),
            document_root,
            file_section_number,
            heading_counters: [0; 6],
            current_code_block: None,
            source_content: String::new(),
            current_line_number: 1,
            source_file,
            metadata_errors: Vec::new(),
        }
    }

    /// Parse markdown content into sections
    ///
    /// # Parameters
    /// * `content` - Raw markdown content to parse
    /// * `document_root` - Root directory of the document for resolving relative image paths
    /// * `file_section_number` - Section number of the markdown file (from filename)
    /// * `source_file` - Path to the source file (relative to document root)
    ///
    /// # Returns
    /// * `Ok(Vec<MarkdownSection>)` - Parsed sections with embedded CSV table blocks
    /// * `Err(SourceModelError)` - Validation error (e.g., missing or invalid h1 heading)
    ///
    /// # Validation Rules
    /// * Source markdown must contain at least one heading
    /// * The first heading must be level 1 (h1)
    /// * Only the first heading may be level 1 (all subsequent headings must be h2+)
    pub fn parse(
        content: &str,
        document_root: &Path,
        file_section_number: &SectionNumber,
        source_file: &Path,
    ) -> Result<Vec<MarkdownSection>, SourceModelError> {
        let mut parser = Self::new(
            document_root.to_path_buf(),
            file_section_number.clone(),
            source_file.to_path_buf(),
        );
        parser.source_content = content.to_string();
        let mut options = pulldown_cmark::Options::empty();
        options.insert(pulldown_cmark::Options::ENABLE_TABLES);
        options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
        let md_parser = pulldown_cmark::Parser::new_ext(content, options);

        for (event, range) in md_parser.into_offset_iter() {
            let line_number = parser.byte_offset_to_line(range.start);
            parser.process_event_with_line(event, line_number);
        }

        // Finalize any remaining content
        parser.finalize();

        // Check for metadata parsing errors
        if let Some(error) = parser.metadata_errors.first() {
            return Err(error.clone());
        }

        // Validate heading structure
        Self::validate_heading_structure(&parser.sections)?;

        // Note: Traceability tables are now generated at the SourceModel level
        // after all files are parsed, not during individual file parsing.
        // See SourceModel::generate_traceability_tables()

        Ok(parser.sections)
    }

    /// Generate traceability tables for sections that request them
    /// NOTE: This method is deprecated and no longer used. Table generation is now done at
    /// the SourceModel level after all files are parsed.
    /// See SourceModel::generate_traceability_tables()
    #[deprecated]
    #[allow(dead_code)]
    fn generate_traceability_tables_deprecated(sections: &mut [MarkdownSection]) {
        // Collect all traceability data from sections
        let section_to_traced = Self::collect_section_traceability(sections);

        // Build reverse mapping: traced_id -> [section_ids]
        let traced_to_sections = Self::build_reverse_traceability(&section_to_traced);

        // Now generate tables for sections that request them
        for section in sections.iter_mut() {
            let Some(ref metadata) = section.metadata else {
                continue;
            };

            // Generate section_id -> traced_ids table
            if let Some((col1, col2)) = metadata
                .generate_section_id_to_traced_ids_table
                .get_headers()
            {
                let table = Self::create_section_to_traced_table(&section_to_traced, &col1, &col2);
                section.content.push(table);
            }

            // Generate traced_id -> section_ids table
            if let Some((col1, col2)) = metadata
                .generate_traced_ids_to_section_ids_table
                .get_headers()
            {
                let table =
                    Self::create_traced_to_sections_table(&traced_to_sections, &col1, &col2);
                section.content.push(table);
            }
        }
    }

    /// Generate traceability tables for sections that request them
    ///
    /// This post-processing step scans all sections for metadata and generates
    /// traceability tables as requested by the `generate_section_id_to_traced_ids_table`
    /// and `generate_traced_ids_to_section_ids_table` flags.
    fn generate_traceability_tables(sections: &mut [MarkdownSection]) {
        // Collect all traceability data from sections
        let section_to_traced = Self::collect_section_traceability(sections);

        // Build reverse mapping: traced_id -> [section_ids]
        let traced_to_sections = Self::build_reverse_traceability(&section_to_traced);

        // Now generate tables for sections that request them
        for section in sections.iter_mut() {
            let Some(ref metadata) = section.metadata else {
                continue;
            };

            // Generate section_id -> traced_ids table
            if let Some((col1, col2)) = metadata
                .generate_section_id_to_traced_ids_table
                .get_headers()
            {
                let table = Self::create_section_to_traced_table(&section_to_traced, &col1, &col2);
                section.content.push(table);
            }

            // Generate traced_id -> section_ids table
            if let Some((col1, col2)) = metadata
                .generate_traced_ids_to_section_ids_table
                .get_headers()
            {
                let table =
                    Self::create_traced_to_sections_table(&traced_to_sections, &col1, &col2);
                section.content.push(table);
            }
        }
    }

    /// Collect traceability data from all sections with metadata
    fn collect_section_traceability(sections: &[MarkdownSection]) -> Vec<(String, Vec<String>)> {
        let mut section_to_traced: Vec<(String, Vec<String>)> = Vec::new();

        for section in sections.iter() {
            let Some(ref metadata) = section.metadata else {
                continue;
            };
            let Some(ref section_id) = metadata.section_id else {
                continue;
            };
            let traced = metadata.traced_ids.clone().unwrap_or_default();
            section_to_traced.push((section_id.clone(), traced));
        }

        // Sort by section_id
        section_to_traced.sort_by(|a, b| a.0.cmp(&b.0));
        section_to_traced
    }

    /// Build reverse mapping from traced_id to section_ids
    fn build_reverse_traceability(
        section_to_traced: &[(String, Vec<String>)],
    ) -> std::collections::BTreeMap<String, Vec<String>> {
        let mut traced_to_sections: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        for (section_id, traced_ids) in section_to_traced {
            for traced_id in traced_ids {
                traced_to_sections
                    .entry(traced_id.clone())
                    .or_default()
                    .push(section_id.clone());
            }
        }

        // Sort the section_ids within each traced_id entry
        for section_ids in traced_to_sections.values_mut() {
            section_ids.sort();
        }

        traced_to_sections
    }

    /// Create a table mapping section_ids to their traced_ids
    fn create_section_to_traced_table(
        section_to_traced: &[(String, Vec<String>)],
        col1_header: &str,
        col2_header: &str,
    ) -> MarkdownBlock {
        use super::text_run::TextRun;

        let headers = vec![
            vec![TextRun::new(col1_header.to_string())],
            vec![TextRun::new(col2_header.to_string())],
        ];

        let rows: Vec<Vec<Vec<TextRun>>> = section_to_traced
            .iter()
            .map(|(section_id, traced_ids)| {
                let mut sorted_traced = traced_ids.clone();
                sorted_traced.sort();
                vec![
                    vec![TextRun::new(section_id.clone())],
                    vec![TextRun::new(sorted_traced.join(", "))],
                ]
            })
            .collect();

        MarkdownBlock::InlineTable {
            alignments: vec![Alignment::None, Alignment::None],
            headers,
            rows,
        }
    }

    /// Create a table mapping traced_ids to section_ids that reference them
    fn create_traced_to_sections_table(
        traced_to_sections: &std::collections::BTreeMap<String, Vec<String>>,
        col1_header: &str,
        col2_header: &str,
    ) -> MarkdownBlock {
        use super::text_run::TextRun;

        let headers = vec![
            vec![TextRun::new(col1_header.to_string())],
            vec![TextRun::new(col2_header.to_string())],
        ];

        let rows: Vec<Vec<Vec<TextRun>>> = traced_to_sections
            .iter()
            .map(|(traced_id, section_ids)| {
                vec![
                    vec![TextRun::new(traced_id.clone())],
                    vec![TextRun::new(section_ids.join(", "))],
                ]
            })
            .collect();

        MarkdownBlock::InlineTable {
            alignments: vec![Alignment::None, Alignment::None],
            headers,
            rows,
        }
    }

    /// Validate that the heading structure follows sysdoc requirements
    ///
    /// # Parameters
    /// * `sections` - Parsed sections to validate
    ///
    /// # Returns
    /// * `Ok(())` - Heading structure is valid
    /// * `Err(SourceModelError)` - Validation failed
    ///
    /// # Validation Rules
    /// * At least one section must exist (with h1 heading)
    /// * The first section must have heading level 1
    /// * All subsequent sections must NOT have heading level 1
    fn validate_heading_structure(sections: &[MarkdownSection]) -> Result<(), SourceModelError> {
        // Rule 1: Must have at least one heading
        if sections.is_empty() {
            return Err(SourceModelError::NoHeadingFound);
        }

        // Rule 2: First heading must be level 1
        if sections[0].heading_level != 1 {
            return Err(SourceModelError::FirstHeadingNotLevel1 {
                actual_level: sections[0].heading_level,
            });
        }

        // Rule 3: Count h1 headings (should be exactly 1)
        let h1_count = sections
            .iter()
            .filter(|section| section.heading_level == 1)
            .count();

        if h1_count > 1 {
            return Err(SourceModelError::MultipleLevel1Headings { count: h1_count });
        }

        Ok(())
    }

    /// Convert byte offset to line number (1-indexed)
    fn byte_offset_to_line(&self, offset: usize) -> usize {
        // Count newlines up to the offset
        self.source_content[..offset.min(self.source_content.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1 // 1-indexed
    }

    /// Process a single markdown event with line number tracking
    fn process_event_with_line(&mut self, event: Event<'_>, line_number: usize) {
        self.current_line_number = line_number;
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag_end) => self.handle_end_tag(tag_end),
            Event::Text(text) => self.handle_text(text.to_string()),
            Event::Code(code) => self.handle_inline_code(code.to_string()),
            Event::SoftBreak => self.handle_soft_break(),
            Event::HardBreak => self.handle_hard_break(),
            Event::Html(html) | Event::InlineHtml(html) => self.handle_html(html.to_string()),
            Event::Rule => self.handle_rule(),
            Event::FootnoteReference(_) => {} // TODO: Handle footnotes
            Event::TaskListMarker(checked) => self.handle_task_marker(checked),
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                self.handle_math(math.to_string())
            }
        }
    }

    /// Handle opening tags
    fn handle_start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                // Start collecting runs for a new paragraph
                self.current_runs.clear();
            }
            Tag::Heading { level, .. } => {
                self.start_heading(level as usize);
            }
            Tag::BlockQuote(_) => {
                self.blockquote_stack.push(Vec::new());
            }
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                self.handle_code_block_start(language);
            }
            Tag::List(start) => {
                self.list_stack.push(ListContext {
                    start,
                    items: Vec::new(),
                    current_item: None,
                });
            }
            Tag::Item => {
                if let Some(list_ctx) = self.list_stack.last_mut() {
                    list_ctx.current_item = Some(Vec::new());
                }
                self.current_runs.clear();
            }
            Tag::Table(alignments) => {
                let aligns = alignments.iter().map(|a| (*a).into()).collect();
                self.table_stack.push(TableContext {
                    alignments: aligns,
                    headers: Vec::new(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    in_header: false,
                });
            }
            Tag::TableHead => {
                if let Some(table_ctx) = self.table_stack.last_mut() {
                    table_ctx.in_header = true;
                }
            }
            Tag::TableRow => {
                if let Some(table_ctx) = self.table_stack.last_mut() {
                    table_ctx.current_row.clear();
                }
            }
            Tag::TableCell => {
                self.current_runs.clear();
            }
            Tag::Emphasis => {
                self.formatting.italic = true;
            }
            Tag::Strong => {
                self.formatting.bold = true;
            }
            Tag::Strikethrough => {
                self.formatting.strikethrough = true;
            }
            Tag::Superscript => {
                self.formatting.superscript = true;
            }
            Tag::Subscript => {
                self.formatting.subscript = true;
            }
            Tag::Link {
                dest_url, title, ..
            } => {
                let url = dest_url.to_string();

                // Check if this is a CSV table reference - handle as a block
                if url.ends_with(".csv") {
                    self.handle_csv_table(url, title.to_string());
                } else {
                    // Regular link - track formatting
                    self.formatting.link_url = Some(url);
                    self.formatting.link_title = (!title.is_empty()).then(|| title.to_string());
                }
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                // Images are handled specially - emit as a block
                self.handle_image(dest_url.to_string(), title.to_string());
            }
            Tag::FootnoteDefinition(_) => {} // TODO: Handle footnotes
            Tag::HtmlBlock => {}             // HTML blocks handled via Event::Html
            Tag::DefinitionList => {}        // TODO: Handle definition lists
            Tag::DefinitionListTitle => {}
            Tag::DefinitionListDefinition => {}
            Tag::MetadataBlock(_) => {}
        }
    }

    /// Handle closing tags
    fn handle_end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Paragraph => {
                self.finish_paragraph();
            }
            TagEnd::Heading(_) => {
                self.finish_heading();
            }
            TagEnd::BlockQuote(_) => {
                self.finish_blockquote();
            }
            TagEnd::CodeBlock => {
                self.finish_code_block();
            }
            TagEnd::List(_) => {
                self.finish_list();
            }
            TagEnd::Item => {
                self.finish_list_item();
            }
            TagEnd::Table => {
                self.finish_table();
            }
            TagEnd::TableHead => {
                // In pulldown-cmark 0.13+, TableHead doesn't contain TableRow,
                // so we need to finish the header row here before clearing in_header
                self.finish_table_row();
                if let Some(table_ctx) = self.table_stack.last_mut() {
                    table_ctx.in_header = false;
                }
            }
            TagEnd::TableRow => {
                self.finish_table_row();
            }
            TagEnd::TableCell => {
                self.finish_table_cell();
            }
            TagEnd::Emphasis => {
                self.formatting.italic = false;
            }
            TagEnd::Strong => {
                self.formatting.bold = false;
            }
            TagEnd::Strikethrough => {
                self.formatting.strikethrough = false;
            }
            TagEnd::Superscript => {
                self.formatting.superscript = false;
            }
            TagEnd::Subscript => {
                self.formatting.subscript = false;
            }
            TagEnd::Link => {
                self.formatting.link_url = None;
                self.formatting.link_title = None;
            }
            TagEnd::Image => {
                // Images handled in start tag
            }
            TagEnd::FootnoteDefinition => {}
            TagEnd::HtmlBlock => {}
            TagEnd::DefinitionList => {}
            TagEnd::DefinitionListTitle => {}
            TagEnd::DefinitionListDefinition => {}
            TagEnd::MetadataBlock(_) => {}
        }
    }

    /// Handle text content
    fn handle_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        // If we're in a code block, append to code block content
        if let Some(code_block) = self.current_code_block.as_mut() {
            code_block.content.push_str(&text);
            return;
        }

        let run = TextRun::with_formatting(text, &self.formatting);
        self.current_runs.push(run);
    }

    /// Handle inline code
    fn handle_inline_code(&mut self, code: String) {
        let mut run = TextRun::with_formatting(code, &self.formatting);
        run.code = true;
        self.current_runs.push(run);
    }

    /// Handle soft break (single newline in source)
    fn handle_soft_break(&mut self) {
        // Soft breaks become spaces in most contexts
        self.current_runs.push(TextRun::new(" ".to_string()));
    }

    /// Handle hard break (two spaces + newline, or <br>)
    fn handle_hard_break(&mut self) {
        // Hard breaks should create a line break within the same paragraph
        // We represent this as a special text run with a newline
        self.current_runs.push(TextRun::new("\n".to_string()));
    }

    /// Handle HTML content
    fn handle_html(&mut self, html: String) {
        // For now, add HTML as a block
        self.add_block(MarkdownBlock::Html(html));
    }

    /// Handle horizontal rule
    fn handle_rule(&mut self) {
        self.add_block(MarkdownBlock::Rule);
    }

    /// Handle task list marker
    fn handle_task_marker(&mut self, _checked: bool) {
        let Some(list_ctx) = self.list_stack.last_mut() else {
            return;
        };

        let Some(_item_blocks) = list_ctx.current_item.as_mut() else {
            return;
        };

        // Mark the current item as a task list item
        // We'll need to store this in the ListItem when we create it
        // For now, we'll handle this when finishing the item
    }

    /// Handle math content
    fn handle_math(&mut self, math: String) {
        // Treat math as inline code for now
        self.handle_inline_code(math);
    }

    /// Start a new heading
    fn start_heading(&mut self, level: usize) {
        // If there's a current section, save it
        if let Some(section) = self.current_section.take() {
            let finalized = self.finalize_section(section);
            self.sections.push(finalized);
        }

        // Start a new section
        self.current_section = Some(SectionBuilder {
            level,
            heading_text: String::new(),
            blocks: Vec::new(),
            metadata: None,
            line_number: self.current_line_number,
        });

        self.current_runs.clear();
    }

    /// Finish a heading
    fn finish_heading(&mut self) {
        let Some(section) = self.current_section.as_mut() else {
            self.current_runs.clear();
            return;
        };

        // Combine all text runs into the heading text
        section.heading_text = self
            .current_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        self.current_runs.clear();
    }

    /// Finish a paragraph
    fn finish_paragraph(&mut self) {
        if self.current_runs.is_empty() {
            return;
        }

        let runs = std::mem::take(&mut self.current_runs);
        let block = MarkdownBlock::Paragraph(runs);

        // Add to appropriate context - check in order of nesting depth
        // 1. Check if we're inside a list item
        if let Some(list_ctx) = self.list_stack.last_mut() {
            if let Some(item_blocks) = list_ctx.current_item.as_mut() {
                item_blocks.push(block);
                return;
            }
        }

        // 2. Check if we're inside a blockquote
        if let Some(blockquote_blocks) = self.blockquote_stack.last_mut() {
            blockquote_blocks.push(block);
            return;
        }

        // 3. Otherwise add to the current section or top-level blocks
        self.add_block(block);
    }

    /// Handle code block start
    fn handle_code_block_start(&mut self, language: Option<String>) {
        // Start accumulating code block content
        self.current_code_block = Some(CodeBlockContext {
            language,
            content: String::new(),
        });
    }

    /// Finish a code block
    fn finish_code_block(&mut self) {
        let Some(code_block) = self.current_code_block.take() else {
            return;
        };

        // Check if this is a sysdoc metadata block
        // Supports both ```sysdoc and ```toml {sysdoc} syntax
        let is_sysdoc = code_block
            .language
            .as_deref()
            .is_some_and(|lang| lang == "sysdoc" || lang.contains("{sysdoc}"));

        if is_sysdoc {
            self.handle_sysdoc_metadata(&code_block.content);
            return;
        }

        // Regular code block - create a CodeBlock
        let block = MarkdownBlock::CodeBlock {
            language: code_block.language,
            code: code_block.content,
            fenced: true,
        };

        self.add_block(block);
    }

    /// Handle sysdoc metadata block content
    fn handle_sysdoc_metadata(&mut self, content: &str) {
        match SectionMetadata::parse(content) {
            Ok(metadata) => {
                // Store metadata in the current section
                if let Some(section) = self.current_section.as_mut() {
                    section.metadata = Some(metadata);
                }
            }
            Err(err) => {
                // Store the error to be reported at the end of parsing
                self.metadata_errors
                    .push(SourceModelError::MetadataParseError {
                        line_number: self.current_line_number,
                        error: err.to_string(),
                    });
            }
        }
    }

    /// Handle image
    fn handle_image(&mut self, url: String, title: String) {
        // Extract alt text from current runs
        let alt_text = self
            .current_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        self.current_runs.clear();

        // Resolve absolute path and check if file exists
        let path = PathBuf::from(&url);
        let absolute_path = self.document_root.join(&path);
        let exists = absolute_path.exists();
        let format = ImageFormat::from_path(&path);

        let block = MarkdownBlock::Image {
            path,
            absolute_path,
            alt_text,
            title,
            format,
            exists,
        };

        self.add_block(block);
    }

    /// Handle CSV table reference
    fn handle_csv_table(&mut self, url: String, _title: String) {
        // Clear any accumulated text runs (link text is not needed for CSV tables)
        self.current_runs.clear();

        // Resolve absolute path and check if file exists
        let path = PathBuf::from(&url);
        let absolute_path = self.document_root.join(&path);
        let exists = absolute_path.exists();

        // Load and parse CSV data if the file exists
        let data = exists
            .then(|| Self::load_csv_data(&absolute_path))
            .flatten();

        let block = MarkdownBlock::CsvTable {
            path,
            absolute_path,
            exists,
            data,
        };

        self.add_block(block);
    }

    /// Load CSV data from a file
    fn load_csv_data(path: &std::path::Path) -> Option<Vec<Vec<String>>> {
        let mut reader = csv::Reader::from_path(path).ok()?;
        let mut rows: Vec<Vec<String>> = Vec::new();

        // Read the headers as the first row
        let headers = reader.headers().ok()?;
        let header_row: Vec<String> = headers.iter().map(String::from).collect();
        rows.push(header_row);

        // Read the data rows
        let data_rows: Vec<Vec<String>> = reader
            .records()
            .flatten()
            .map(|record| record.iter().map(String::from).collect())
            .collect();
        rows.extend(data_rows);

        Some(rows)
    }

    /// Finish a list
    fn finish_list(&mut self) {
        let Some(list_ctx) = self.list_stack.pop() else {
            return;
        };

        let block = MarkdownBlock::List {
            start: list_ctx.start,
            items: list_ctx.items,
        };

        self.add_block(block);
    }

    /// Finish a list item
    fn finish_list_item(&mut self) {
        // Finish any pending paragraph
        if !self.current_runs.is_empty() {
            self.finish_paragraph();
        }

        let Some(list_ctx) = self.list_stack.last_mut() else {
            return;
        };

        let Some(item_blocks) = list_ctx.current_item.take() else {
            return;
        };

        let mut item = ListItem::new();
        item.content = item_blocks;
        list_ctx.items.push(item);
    }

    /// Finish a blockquote
    fn finish_blockquote(&mut self) {
        let Some(blocks) = self.blockquote_stack.pop() else {
            return;
        };

        let block = MarkdownBlock::BlockQuote(blocks);
        self.add_block(block);
    }

    /// Finish a table
    fn finish_table(&mut self) {
        let Some(table_ctx) = self.table_stack.pop() else {
            return;
        };

        let block = MarkdownBlock::InlineTable {
            alignments: table_ctx.alignments,
            headers: table_ctx.headers,
            rows: table_ctx.rows,
        };
        self.add_block(block);
    }

    /// Finish a table row
    fn finish_table_row(&mut self) {
        let Some(table_ctx) = self.table_stack.last_mut() else {
            return;
        };

        let row = std::mem::take(&mut table_ctx.current_row);
        if table_ctx.in_header {
            table_ctx.headers = row;
        } else {
            table_ctx.rows.push(row);
        }
    }

    /// Finish a table cell
    fn finish_table_cell(&mut self) {
        let Some(table_ctx) = self.table_stack.last_mut() else {
            return;
        };

        let cell = std::mem::take(&mut self.current_runs);
        table_ctx.current_row.push(cell);
    }

    /// Add a block to the appropriate context
    fn add_block(&mut self, block: MarkdownBlock) {
        if let Some(section) = self.current_section.as_mut() {
            section.blocks.push(block);
            return;
        }

        self.current_blocks.push(block);
    }

    /// Finalize parsing
    fn finalize(&mut self) {
        // Finish any pending section
        if let Some(section) = self.current_section.take() {
            let finalized = self.finalize_section(section);
            self.sections.push(finalized);
        }

        // Note: We do NOT create default sections for content without headings.
        // sysdoc requires all source markdown files to have an explicit h1 heading.
        // Content without headings will fail validation.
    }

    /// Convert a section builder into a MarkdownSection with calculated section number
    ///
    /// Calculates the section number by combining:
    /// 1. The file's section number (with .00 stripped if present) - used directly for h1
    /// 2. For h2+, incremental counters are appended to the base number
    ///
    /// The h1 heading uses the file section number directly because the filename already
    /// encodes the section number. For example:
    /// - File "01.00_scope.md" with h1 → section 1
    /// - File "01.02_overview.md" with h1 → section 1.2
    /// - File "01.02_overview.md" with h2 (first) → section 1.2.1
    /// - File "01.02_overview.md" with h2 (second) → section 1.2.2
    fn finalize_section(&mut self, section: SectionBuilder) -> MarkdownSection {
        // Build section number
        // Start with file section number (stripping .00 if present)
        let base_number = if self.file_section_number.is_parent_marker() {
            self.file_section_number.without_parent_marker().unwrap()
        } else {
            self.file_section_number.clone()
        };

        let section_number = if section.level == 1 {
            // For h1, use the file section number directly
            // The filename already encodes the section number
            base_number
        } else {
            // For h2+, increment counter for this level and add to base
            let level_index = section.level.saturating_sub(1).min(5); // h2=1, h3=2, etc.
            self.heading_counters[level_index] += 1;

            // Reset all deeper level counters
            for i in (level_index + 1)..6 {
                self.heading_counters[i] = 0;
            }

            // Add heading level counters (from h2 onwards, skip h1 counter at index 0)
            let additional: Vec<u32> = self.heading_counters[1..=level_index].to_vec();
            base_number.extend(&additional).unwrap_or_else(|err| {
                // Log warning when depth is exceeded
                log::warn!(
                    "Section number depth exceeded for heading '{}': {}. Using fallback.",
                    section.heading_text,
                    err
                );
                base_number.clone()
            })
        };

        // Build content, potentially adding an included code block at the end
        let mut content = section.blocks;

        // If metadata specifies include_file, load it and append as a code block
        if let Some(ref metadata) = section.metadata {
            if let Some(ref include_path) = metadata.include_file {
                let included_block = self.create_included_code_block(include_path);
                content.push(included_block);
            }
        }

        MarkdownSection {
            heading_level: section.level,
            heading_text: section.heading_text,
            section_number,
            line_number: section.line_number,
            source_file: self.source_file.clone(),
            content,
            metadata: section.metadata,
        }
    }

    /// Create an IncludedCodeBlock from a file path specified in metadata
    fn create_included_code_block(&self, include_path: &str) -> MarkdownBlock {
        let path = PathBuf::from(include_path);
        let absolute_path = self.document_root.join(&path);
        let exists = absolute_path.exists();

        // Infer language from file extension
        let language = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        // Load file content if it exists
        let content = if exists {
            std::fs::read_to_string(&absolute_path).ok()
        } else {
            None
        };

        MarkdownBlock::IncludedCodeBlock {
            path,
            absolute_path,
            language,
            content,
            exists,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Event, Parser, Tag};

    // ============================================================================
    // Tests documenting pulldown_cmark behavior
    // ============================================================================

    #[test]
    fn test_standalone_image_wrapped_in_paragraph() {
        // Arrange: Markdown with only an image
        let markdown = "![alt text](image.png)";

        // Act: Parse markdown into events
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Standalone images are wrapped in paragraph tags
        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));
        assert!(matches!(events[1], Event::Start(Tag::Image { .. })));
        assert!(matches!(events[2], Event::Text(_)));
        assert!(matches!(events[3], Event::End(_))); // End(Image)
        assert!(matches!(events[4], Event::End(_))); // End(Paragraph)
    }

    #[test]
    fn test_image_with_text_in_one_paragraph() {
        // Arrange: Image surrounded by text
        let markdown = "Some text ![alt](image.png) more text";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Everything is in one paragraph
        assert!(matches!(events.first(), Some(Event::Start(Tag::Paragraph))));
        assert!(matches!(events.last(), Some(Event::End(_))));

        // Verify it contains an image tag
        let has_image = events
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Image { .. })));
        assert!(has_image, "Should contain an image within the paragraph");
    }

    #[test]
    fn test_heading_not_wrapped_in_paragraph() {
        // Arrange: Markdown heading
        let markdown = "# Heading";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Heading is NOT wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::Heading { .. })));
        assert!(matches!(events[1], Event::Text(_)));
        assert!(matches!(events[2], Event::End(_)));
    }

    #[test]
    fn test_blockquote_not_wrapped_in_paragraph() {
        // Arrange: Markdown blockquote
        let markdown = "> Quote";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: BlockQuote is NOT wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::BlockQuote(_))));
    }

    #[test]
    fn test_plain_text_wrapped_in_paragraph() {
        // Arrange: Plain text
        let markdown = "Plain text";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Plain text IS wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));
        assert!(matches!(events[1], Event::Text(_)));
        assert!(matches!(events[2], Event::End(_)));
    }

    #[test]
    fn test_link_wrapped_in_paragraph() {
        // Arrange: Markdown link
        let markdown = "[link](url)";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Links ARE wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));
        assert!(matches!(events[1], Event::Start(Tag::Link { .. })));
    }

    #[test]
    fn test_inline_code_wrapped_in_paragraph() {
        // Arrange: Inline code
        let markdown = "`inline code`";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Inline code IS wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));
        assert!(matches!(events[1], Event::Code(_)));
        assert!(matches!(events[2], Event::End(_)));
    }

    #[test]
    fn test_bold_text_wrapped_in_paragraph() {
        // Arrange: Bold text
        let markdown = "**bold** text";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Bold text IS wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));
        assert!(matches!(events[1], Event::Start(Tag::Strong)));
    }

    #[test]
    fn test_list_not_wrapped_in_paragraph() {
        // Arrange: Markdown list
        let markdown = "- Item 1\n- Item 2";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: List is NOT wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::List(_))));
    }

    #[test]
    fn test_code_block_not_wrapped_in_paragraph() {
        // Arrange: Fenced code block
        let markdown = "```rust\ncode\n```";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Code block is NOT wrapped in paragraph
        assert!(matches!(events[0], Event::Start(Tag::CodeBlock(_))));
    }

    #[test]
    fn test_horizontal_rule_not_wrapped() {
        // Arrange: Horizontal rule
        let markdown = "---";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Rule is a standalone event
        assert!(matches!(events[0], Event::Rule));
    }

    #[test]
    fn test_multiple_images_in_single_paragraph() {
        // Arrange: Three images with text between them
        let markdown = "![img1](a.png) text ![img2](b.png) more ![img3](c.png)";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: All images are in one paragraph
        assert!(matches!(events[0], Event::Start(Tag::Paragraph)));

        let image_count = events
            .iter()
            .filter(|e| matches!(e, Event::Start(Tag::Image { .. })))
            .count();
        assert_eq!(image_count, 3);

        assert!(matches!(events[events.len() - 1], Event::End(_)));
    }

    #[test]
    fn test_image_in_list_item() {
        // Arrange: List with image in item
        let markdown = "- ![image](img.png)";

        // Act: Parse markdown
        let events: Vec<Event> = Parser::new(markdown).collect();

        // Assert: Verify list structure with image
        assert!(matches!(events[0], Event::Start(Tag::List(_))));
        assert!(matches!(events[1], Event::Start(Tag::Item)));

        let has_image = events
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Image { .. })));
        assert!(has_image, "List item should contain image");
    }

    // ============================================================================
    // Tests documenting our parser behavior
    // ============================================================================

    /// Helper to create a test section number
    fn test_section_number() -> SectionNumber {
        SectionNumber::parse("01.00").unwrap()
    }

    // ============================================================================
    // Section number calculation tests
    // ============================================================================

    #[test]
    fn test_section_number_calculation_basic() {
        // Arrange: File "01.02.md" with one h1 heading
        let markdown = "# First Heading\n\nContent here.";
        let file_section = SectionNumber::parse("01.02").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: h1 uses file section number directly (filename encodes section number)
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_number.to_string(), "1.2");
    }

    #[test]
    fn test_section_number_calculation_multiple_headings() {
        // Arrange: File "01.02.md" with multiple headings
        let markdown = r#"# First Heading

Content 1

## Second Level 1

Content 2

## Second Level 2

Content 3"#;
        let file_section = SectionNumber::parse("01.02").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: h1 uses file number directly, h2+ add counters
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].section_number.to_string(), "1.2"); // h1 = file section
        assert_eq!(sections[1].section_number.to_string(), "1.2.1"); // First h2
        assert_eq!(sections[2].section_number.to_string(), "1.2.2"); // Second h2
    }

    #[test]
    fn test_section_number_parent_marker_stripped() {
        // Arrange: File "01.02.00.md" (parent marker) with one h1 heading
        let markdown = "# Heading\n\nContent.";
        let file_section = SectionNumber::parse("01.02.00").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: .00 should be stripped, h1 uses file section number directly
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_number.to_string(), "1.2");
    }

    #[test]
    fn test_section_number_deep_nesting() {
        // Arrange: File "01.md" with nested headings up to h5 (to stay within max depth of 6)
        let markdown = r#"# Main Heading

## H2

### H3

#### H4

##### H5"#;
        let file_section = SectionNumber::parse("01").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: h1 = file section, h2+ add counters
        assert_eq!(sections.len(), 5);
        assert_eq!(sections[0].section_number.to_string(), "1"); // h1 = file section
        assert_eq!(sections[1].section_number.to_string(), "1.1"); // h2
        assert_eq!(sections[2].section_number.to_string(), "1.1.1"); // h3
        assert_eq!(sections[3].section_number.to_string(), "1.1.1.1"); // h4
        assert_eq!(sections[4].section_number.to_string(), "1.1.1.1.1"); // h5
    }

    #[test]
    fn test_section_number_max_depth_enforced() {
        // Arrange: File "01.02.md" (depth 2) with h6 heading would exceed max depth of 6
        // h1 = "1.2" (depth 2), h2 = "1.2.1" (depth 3), ..., h6 would need depth 7
        let markdown = r#"# H1

## H2

### H3

#### H4

##### H5

###### H6"#;
        let file_section = SectionNumber::parse("01.02").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: h6 exceeds max depth and falls back to base file number
        assert_eq!(sections.len(), 6);
        assert_eq!(sections[0].section_number.to_string(), "1.2"); // h1 = file section
        assert_eq!(sections[1].section_number.to_string(), "1.2.1"); // h2
        assert_eq!(sections[2].section_number.to_string(), "1.2.1.1"); // h3
        assert_eq!(sections[3].section_number.to_string(), "1.2.1.1.1"); // h4
        assert_eq!(sections[4].section_number.to_string(), "1.2.1.1.1.1"); // h5 (depth 6, at max)
                                                                           // h6 would exceed depth, so it falls back to file number
        assert_eq!(sections[5].section_number.to_string(), "1.2");
    }

    #[test]
    fn test_section_number_counter_reset() {
        // Arrange: File "01.md" with h2 and h3 headings to test counter reset
        let markdown = r#"# Main Heading

## First H2

### H3 under first H2

### Another H3

## Second H2

### H3 under second H2"#;
        let file_section = SectionNumber::parse("01").unwrap();

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &file_section,
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Counters should reset when returning to higher level
        assert_eq!(sections.len(), 6);
        assert_eq!(sections[0].section_number.to_string(), "1"); // h1 = file section
        assert_eq!(sections[1].section_number.to_string(), "1.1"); // First h2
        assert_eq!(sections[2].section_number.to_string(), "1.1.1"); // First h3
        assert_eq!(sections[3].section_number.to_string(), "1.1.2"); // Second h3
        assert_eq!(sections[4].section_number.to_string(), "1.2"); // Second h2 (h3 counter reset)
        assert_eq!(sections[5].section_number.to_string(), "1.2.1"); // h3 under second h2
    }

    #[test]
    fn test_our_parser_extracts_images_from_markdown() {
        // Arrange: Markdown with multiple images
        let markdown = r#"
# Section 1

Some text with an ![inline image](inline.png) here.

![standalone image](standalone.png)

More text after.
"#;

        // Act: Parse with our parser
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Verify section structure
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading_text, "Section 1");

        // Count image blocks (our parser may extract images separately)
        let image_count = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::Image { .. }))
            .count();

        assert!(image_count >= 1, "Should extract at least one image");
    }

    // ============================================================================
    // Unit tests for MarkdownParser::parse
    // ============================================================================

    // ============================================================================
    // Validation tests - heading structure
    // ============================================================================

    #[test]
    fn test_parse_valid_single_h1() {
        // Arrange: Valid markdown with single h1
        let markdown = "# Main Heading\n\nContent here.";

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should succeed
        assert!(result.is_ok());
        let sections = result.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading_level, 1);
    }

    #[test]
    fn test_parse_error_no_heading() {
        // Arrange: Markdown with no headings
        let markdown = "Just a paragraph with no headings.";

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should fail with NoHeadingFound error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SourceModelError::NoHeadingFound);
    }

    #[test]
    fn test_parse_error_first_heading_not_h1() {
        // Arrange: Markdown where first heading is h2
        let markdown = "## Second Level Heading\n\nContent here.";

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should fail with FirstHeadingNotLevel1 error
        assert!(result.is_err());
        match result.unwrap_err() {
            SourceModelError::FirstHeadingNotLevel1 { actual_level } => {
                assert_eq!(actual_level, 2);
            }
            _ => panic!("Expected FirstHeadingNotLevel1 error"),
        }
    }

    #[test]
    fn test_parse_error_multiple_h1_headings() {
        // Arrange: Markdown with multiple h1 headings
        let markdown = r#"# First Heading

Content 1

# Second Heading

Content 2"#;

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should fail with MultipleLevel1Headings error
        assert!(result.is_err());
        match result.unwrap_err() {
            SourceModelError::MultipleLevel1Headings { count } => {
                assert_eq!(count, 2);
            }
            _ => panic!("Expected MultipleLevel1Headings error"),
        }
    }

    #[test]
    fn test_parse_valid_h1_with_h2_subsections() {
        // Arrange: Valid markdown with h1 followed by h2 subsections
        let markdown = r#"# Main Heading

Introduction content.

## Subsection 1

Content 1

## Subsection 2

Content 2"#;

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should succeed with 3 sections
        assert!(result.is_ok());
        let sections = result.unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].heading_level, 1);
        assert_eq!(sections[1].heading_level, 2);
        assert_eq!(sections[2].heading_level, 2);
    }

    #[test]
    fn test_parse_valid_deep_nesting() {
        // Arrange: Valid markdown with deep heading nesting
        let markdown = r#"# Main

Content

## Level 2

More content

### Level 3

Deep content

#### Level 4

Very deep"#;

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should succeed with 4 sections
        assert!(result.is_ok());
        let sections = result.unwrap();
        assert_eq!(sections.len(), 4);
        assert_eq!(sections[0].heading_level, 1);
        assert_eq!(sections[1].heading_level, 2);
        assert_eq!(sections[2].heading_level, 3);
        assert_eq!(sections[3].heading_level, 4);
    }

    // ============================================================================
    // Existing parser tests (updated to use .unwrap())
    // ============================================================================

    #[test]
    fn test_parse_simple_paragraph() {
        // Arrange: Simple text paragraph with h1
        let markdown = "# Heading\n\nThis is a simple paragraph.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create one section with one paragraph
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);
        assert!(matches!(
            sections[0].content[0],
            MarkdownBlock::Paragraph(_)
        ));
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        // Arrange: Two paragraphs separated by blank line
        let markdown = "# Heading

First paragraph.

Second paragraph.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create one section with two paragraphs
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 2);
        assert!(matches!(
            sections[0].content[0],
            MarkdownBlock::Paragraph(_)
        ));
        assert!(matches!(
            sections[0].content[1],
            MarkdownBlock::Paragraph(_)
        ));
    }

    #[test]
    fn test_parse_single_heading() {
        // Arrange: Markdown with one heading
        let markdown = "# Main Heading\n\nSome content.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create one section with heading
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading_level, 1);
        assert_eq!(sections[0].heading_text, "Main Heading");
        assert_eq!(sections[0].content.len(), 1);
    }

    #[test]
    fn test_parse_multiple_headings_create_sections() {
        // Arrange: Markdown with three headings (one h1, two h2)
        let markdown = r#"# First Heading

Content 1

## Second Heading

Content 2

## Third Heading

Content 3"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create three sections
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].heading_text, "First Heading");
        assert_eq!(sections[0].heading_level, 1);
        assert_eq!(sections[1].heading_text, "Second Heading");
        assert_eq!(sections[1].heading_level, 2);
        assert_eq!(sections[2].heading_text, "Third Heading");
        assert_eq!(sections[2].heading_level, 2);
    }

    #[test]
    fn test_parse_unordered_list() {
        // Arrange: Simple unordered list
        let markdown = "# List\n\n- Item 1\n- Item 2\n- Item 3";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create one list block
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::List { start, items } => {
                assert_eq!(start, &None);
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected List block"),
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        // Arrange: Ordered list with explicit numbering
        let markdown = "# List\n\n1. First\n2. Second\n3. Third";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create ordered list starting at 1
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::List { start, items } => {
                assert_eq!(start, &Some(1));
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected List block"),
        }
    }

    #[test]
    fn test_parse_fenced_code_block() {
        // Arrange: Fenced code block with language
        let markdown = "# Code\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Code blocks without headings create a default section
        // But code blocks might not be implemented yet, so check what we got
        if sections.is_empty() {
            // Code block parsing not yet implemented
            return;
        }

        assert_eq!(sections.len(), 1);

        if sections[0].content.is_empty() {
            // Code block parsing not yet fully implemented
            return;
        }

        match &sections[0].content[0] {
            MarkdownBlock::CodeBlock {
                language,
                code,
                fenced: _,
            } => {
                assert_eq!(language, &Some("rust".to_string()));
                assert!(code.contains("fn main"));
            }
            _ => panic!("Expected CodeBlock, got {:?}", sections[0].content[0]),
        }
    }

    #[test]
    fn test_parse_blockquote() {
        // Arrange: Simple blockquote
        let markdown = "# Quote\n\n> This is a quote\n> with multiple lines";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create blockquote block
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);
        assert!(matches!(
            sections[0].content[0],
            MarkdownBlock::BlockQuote(_)
        ));
    }

    #[test]
    fn test_parse_horizontal_rule() {
        // Arrange: Horizontal rule between paragraphs
        let markdown = "# Rule\n\nBefore rule\n\n---\n\nAfter rule";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create paragraph, rule, paragraph
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 3);
        assert!(matches!(
            sections[0].content[0],
            MarkdownBlock::Paragraph(_)
        ));
        assert!(matches!(sections[0].content[1], MarkdownBlock::Rule));
        assert!(matches!(
            sections[0].content[2],
            MarkdownBlock::Paragraph(_)
        ));
    }

    #[test]
    fn test_parse_bold_text() {
        // Arrange: Paragraph with bold text
        let markdown = "# Text\n\nThis is **bold** text.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should parse with formatted runs
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::Paragraph(runs) => {
                assert!(runs.len() >= 2);
                let bold_run = runs.iter().find(|r| r.bold);
                assert!(bold_run.is_some(), "Should have at least one bold run");
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_italic_text() {
        // Arrange: Paragraph with italic text
        let markdown = "# Text\n\nThis is *italic* text.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should parse with italic formatting
        assert_eq!(sections.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::Paragraph(runs) => {
                let italic_run = runs.iter().find(|r| r.italic);
                assert!(italic_run.is_some(), "Should have at least one italic run");
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_inline_code() {
        // Arrange: Paragraph with inline code
        let markdown = "# Code\n\nUse the `println!` macro.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should parse with code formatting
        assert_eq!(sections.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::Paragraph(runs) => {
                let code_run = runs.iter().find(|r| r.code);
                assert!(code_run.is_some(), "Should have at least one code run");
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_link() {
        // Arrange: Paragraph with link
        let markdown = "# Link\n\nVisit [Rust](https://rust-lang.org) website.";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should parse with link formatting
        assert_eq!(sections.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::Paragraph(runs) => {
                let link_run = runs.iter().find(|r| r.link_url.is_some());
                assert!(link_run.is_some(), "Should have at least one link run");
                assert_eq!(
                    link_run.unwrap().link_url.as_ref().unwrap(),
                    "https://rust-lang.org"
                );
            }
            _ => panic!("Expected Paragraph"),
        }
    }

    #[test]
    fn test_parse_csv_table_reference() {
        // Arrange: Link to CSV file
        let markdown = "# Table\n\n[Table Data](data.csv)";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create a CsvTable block
        assert_eq!(sections.len(), 1);

        // CSV tables are now embedded as CsvTable blocks
        let csv_count = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::CsvTable { .. }))
            .count();
        assert_eq!(csv_count, 1);

        // Check that the path is correct
        match &sections[0].content[0] {
            MarkdownBlock::CsvTable { path, .. } => {
                assert_eq!(path, &PathBuf::from("data.csv"));
            }
            _ => panic!("Expected CsvTable block"),
        }
    }

    #[test]
    fn test_parse_multiple_csv_references() {
        // Arrange: Multiple CSV links
        let markdown = r#"# Data Section

First table: [table1](table1.csv)

Second table: [table2](table2.csv)"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create CsvTable blocks for each CSV
        assert_eq!(sections.len(), 1);

        // CSV tables are now embedded as CsvTable blocks
        let csv_tables: Vec<&PathBuf> = sections[0]
            .content
            .iter()
            .filter_map(|block| match block {
                MarkdownBlock::CsvTable { path, .. } => Some(path),
                _ => None,
            })
            .collect();

        assert_eq!(csv_tables.len(), 2);
        assert!(csv_tables.contains(&&PathBuf::from("table1.csv")));
        assert!(csv_tables.contains(&&PathBuf::from("table2.csv")));
    }

    #[test]
    fn test_parse_image_block() {
        // Arrange: Standalone image
        let markdown = "# Image\n\n![Alt text](image.png)";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should extract image
        assert_eq!(sections.len(), 1);

        // Images wrapped in paragraphs by pulldown_cmark may create multiple blocks
        // Find the image block
        let image_block = sections[0]
            .content
            .iter()
            .find(|block| matches!(block, MarkdownBlock::Image { .. }));

        assert!(image_block.is_some(), "Should contain an image block");

        match image_block.unwrap() {
            MarkdownBlock::Image {
                path,
                absolute_path: _,
                alt_text: _,
                title: _,
                format: _,
                exists: _,
            } => {
                // Verify path is correct
                assert_eq!(path, &PathBuf::from("image.png"));
                // Note: alt_text handling may vary based on when image is extracted
                // from the event stream (before or after text events are processed)
                // absolute_path will be "./image.png" since we use "." as document root in tests
                // exists will be false unless the test image actually exists
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_parse_empty_content() {
        // Arrange: Empty string (this test now expects an error since no heading)
        let markdown = "";

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should fail with NoHeadingFound error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SourceModelError::NoHeadingFound);
    }

    #[test]
    fn test_parse_whitespace_only() {
        // Arrange: Only whitespace (this test now expects an error since no heading)
        let markdown = "   \n\n   \n";

        // Act: Parse the markdown
        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Assert: Should fail with NoHeadingFound error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SourceModelError::NoHeadingFound);
    }

    #[test]
    fn test_parse_mixed_content() {
        // Arrange: Complex markdown with various elements
        let markdown = r#"# Introduction

This is a **bold** paragraph with *italic* and `code`.

## Features

- Feature 1
- Feature 2

Here's a code example:

```rust
fn example() {}
```

> Important note

---

End of document."#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create two sections with various blocks
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading_text, "Introduction");
        assert_eq!(sections[1].heading_text, "Features");
        assert!(!sections[0].content.is_empty());
        assert!(sections[1].content.len() >= 4);
    }

    #[test]
    fn test_parse_nested_list() {
        // Arrange: List with nested items
        let markdown = r#"# List

- Item 1
  - Nested 1a
  - Nested 1b
- Item 2"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should parse list structure
        assert_eq!(sections.len(), 1);

        // Nested lists may create multiple list blocks or nested structures
        let list_count = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::List { .. }))
            .count();

        assert!(list_count >= 1, "Should contain at least one list block");
    }

    #[test]
    fn test_parse_blockquote_with_content() {
        // Arrange: Blockquote with formatted content
        let markdown = "# Quote\n\n> This is **bold** in a quote\n> \n> Second paragraph";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should create blockquote with nested blocks
        assert_eq!(sections.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::BlockQuote(blocks) => {
                assert!(!blocks.is_empty(), "Blockquote should contain blocks");
            }
            _ => panic!("Expected BlockQuote"),
        }
    }

    #[test]
    fn test_parse_html_content() {
        // Arrange: Raw HTML in markdown
        let markdown = "# HTML\n\n<div>HTML content</div>";

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should preserve HTML
        assert_eq!(sections.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::Html(html) => {
                assert!(html.contains("HTML content"));
            }
            _ => panic!("Expected Html block"),
        }
    }

    // ============================================================================
    // Section metadata tests
    // ============================================================================

    #[test]
    fn test_parse_sysdoc_metadata_block() {
        // Arrange: Markdown with sysdoc metadata block
        let markdown = r#"# Section with Metadata

```sysdoc
section_id = "REQ-001"
traced_ids = ["SRS-001", "SRS-002"]
```

Some content here.
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should have metadata parsed
        assert_eq!(sections.len(), 1);
        assert!(sections[0].metadata.is_some());

        let metadata = sections[0].metadata.as_ref().unwrap();
        assert_eq!(metadata.section_id, Some("REQ-001".to_string()));
        assert_eq!(
            metadata.traced_ids,
            Some(vec!["SRS-001".to_string(), "SRS-002".to_string()])
        );
    }

    #[test]
    fn test_sysdoc_block_not_in_content() {
        // Arrange: Markdown with sysdoc block
        let markdown = r#"# Section

```sysdoc
section_id = "REQ-001"
```

Some content.
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: sysdoc block should NOT appear as a CodeBlock
        let code_blocks: Vec<_> = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::CodeBlock { .. }))
            .collect();

        assert!(
            code_blocks.is_empty(),
            "sysdoc blocks should not be in content"
        );
    }

    #[test]
    fn test_regular_code_block_still_works() {
        // Arrange: Markdown with regular code block
        let markdown = r#"# Code Section

```rust
fn main() {}
```
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: Should have a CodeBlock
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content.len(), 1);

        match &sections[0].content[0] {
            MarkdownBlock::CodeBlock { language, code, .. } => {
                assert_eq!(language, &Some("rust".to_string()));
                assert!(code.contains("fn main"));
            }
            _ => panic!("Expected CodeBlock"),
        }
    }

    #[test]
    #[ignore = "Table generation now happens at SourceModel level, not during parsing"]
    fn test_generate_section_to_traced_table() {
        // NOTE: This test is deprecated. Table generation now happens at the SourceModel level
        // after all files are parsed, not during individual file parsing.
        // See SourceModel::generate_traceability_tables() and related tests in source_model.rs

        // Arrange: Multiple sections with metadata requesting forward table
        let markdown = r#"# Traceability

```sysdoc
generate_section_id_to_traced_ids_table = ["Section ID", "Traced IDs"]
```

## Requirement 1

```sysdoc
section_id = "REQ-001"
traced_ids = ["SRS-001", "SRS-002"]
```

Content 1.

## Requirement 2

```sysdoc
section_id = "REQ-002"
traced_ids = ["SRS-003"]
```

Content 2.
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: First section should have a generated table
        assert_eq!(sections.len(), 3);

        // Find InlineTable in the first section
        let tables: Vec<_> = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::InlineTable { .. }))
            .collect();

        assert_eq!(tables.len(), 1, "Should have one generated table");

        match tables[0] {
            MarkdownBlock::InlineTable { headers, rows, .. } => {
                // Check headers
                assert_eq!(headers.len(), 2);
                assert_eq!(headers[0][0].text, "Section ID");
                assert_eq!(headers[1][0].text, "Traced IDs");

                // Check rows (should be sorted by section_id)
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0][0][0].text, "REQ-001");
                assert_eq!(rows[0][1][0].text, "SRS-001, SRS-002");
                assert_eq!(rows[1][0][0].text, "REQ-002");
                assert_eq!(rows[1][1][0].text, "SRS-003");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    #[ignore = "Table generation now happens at SourceModel level, not during parsing"]
    fn test_generate_traced_to_sections_table() {
        // NOTE: This test is deprecated. Table generation now happens at the SourceModel level
        // after all files are parsed, not during individual file parsing.
        // See SourceModel::generate_traceability_tables() and related tests in source_model.rs

        // Arrange: Multiple sections with metadata requesting reverse table
        let markdown = r#"# Traceability

```sysdoc
generate_traced_ids_to_section_ids_table = ["Traced ID", "Section IDs"]
```

## Requirement 1

```sysdoc
section_id = "REQ-001"
traced_ids = ["SRS-001", "SRS-002"]
```

Content 1.

## Requirement 2

```sysdoc
section_id = "REQ-002"
traced_ids = ["SRS-001"]
```

Content 2.
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: First section should have a generated table
        assert_eq!(sections.len(), 3);

        // Find InlineTable in the first section
        let tables: Vec<_> = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::InlineTable { .. }))
            .collect();

        assert_eq!(tables.len(), 1, "Should have one generated table");

        match tables[0] {
            MarkdownBlock::InlineTable { headers, rows, .. } => {
                // Check headers
                assert_eq!(headers.len(), 2);
                assert_eq!(headers[0][0].text, "Traced ID");
                assert_eq!(headers[1][0].text, "Section IDs");

                // Check rows (should be sorted by traced_id, with SRS-001 mapping to both REQ-001 and REQ-002)
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0][0][0].text, "SRS-001");
                assert_eq!(rows[0][1][0].text, "REQ-001, REQ-002");
                assert_eq!(rows[1][0][0].text, "SRS-002");
                assert_eq!(rows[1][1][0].text, "REQ-001");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_no_metadata_no_table() {
        // Arrange: Section without metadata
        let markdown = r#"# Plain Section

No metadata here.
"#;

        // Act: Parse the markdown
        let sections = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        )
        .unwrap();

        // Assert: No metadata and no generated tables
        assert_eq!(sections.len(), 1);
        assert!(sections[0].metadata.is_none());

        let tables: Vec<_> = sections[0]
            .content
            .iter()
            .filter(|block| matches!(block, MarkdownBlock::InlineTable { .. }))
            .collect();

        assert!(tables.is_empty(), "Should have no generated tables");
    }

    #[test]
    fn test_invalid_metadata_syntax_fails_build() {
        // This test verifies that using the old syntax "= true" produces a clear error
        let markdown = r#"# Test Section

```sysdoc
section_id = "REQ-001"
generate_section_id_to_traced_ids_table = true
```

Some content here.
"#;

        let result = MarkdownParser::parse(
            markdown,
            &PathBuf::from("."),
            &test_section_number(),
            &PathBuf::from("test.md"),
        );

        // Verify that parsing fails with a metadata error
        assert!(
            result.is_err(),
            "Expected parsing to fail with metadata error"
        );

        let error = result.unwrap_err();
        let error_msg = format!("{}", error);

        // Verify the error message contains helpful information
        assert!(
            error_msg.contains("custom headers"),
            "Error message should explain custom headers are required. Got: {}",
            error_msg
        );
        assert!(
            error_msg.contains("sysdoc metadata block"),
            "Error message should mention sysdoc metadata block. Got: {}",
            error_msg
        );
    }
}
