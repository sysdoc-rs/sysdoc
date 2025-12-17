//! Markdown event stream parser
//!
//! Converts pulldown-cmark's event stream into structured blocks with formatted text runs.

use super::blocks::{ListItem, MarkdownBlock};
use super::markdown_source::MarkdownSection;
use super::text_run::{TextFormatting, TextRun};
use super::types::Alignment;
use pulldown_cmark::{Event, Tag, TagEnd};
use std::path::PathBuf;

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

    /// CSV table references found
    table_refs: Vec<PathBuf>,
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
}

impl MarkdownParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            formatting: TextFormatting::new(),
            current_runs: Vec::new(),
            current_blocks: Vec::new(),
            list_stack: Vec::new(),
            table_stack: Vec::new(),
            blockquote_stack: Vec::new(),
            current_section: None,
            sections: Vec::new(),
            table_refs: Vec::new(),
        }
    }

    /// Parse markdown content into sections
    pub fn parse(content: &str) -> (Vec<MarkdownSection>, Vec<PathBuf>) {
        let mut parser = Self::new();
        let md_parser = pulldown_cmark::Parser::new(content);

        for event in md_parser {
            parser.process_event(event);
        }

        // Finalize any remaining content
        parser.finalize();

        (parser.sections, parser.table_refs)
    }

    /// Process a single markdown event
    fn process_event(&mut self, event: Event<'_>) {
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

                // Check if this is a CSV table reference
                if url.ends_with(".csv") {
                    self.table_refs.push(PathBuf::from(&url));
                }

                self.formatting.link_url = Some(url);
                self.formatting.link_title = if title.is_empty() {
                    None
                } else {
                    Some(title.to_string())
                };
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
                // Code blocks are handled in their text events
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
            let table_refs = std::mem::take(&mut self.table_refs);
            self.sections
                .push(Self::finalize_section_static(section, table_refs));
        }

        // Start a new section
        self.current_section = Some(SectionBuilder {
            level,
            heading_text: String::new(),
            blocks: Vec::new(),
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
    fn handle_code_block_start(&mut self, _language: Option<String>) {
        // Code blocks will have their content in text events
        // We'll accumulate the text and create the block on end
        self.current_runs.clear();
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

        let block = MarkdownBlock::Image {
            path: PathBuf::from(url),
            alt_text,
            title,
        };

        self.add_block(block);
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

        let block = MarkdownBlock::Table {
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
            let table_refs = std::mem::take(&mut self.table_refs);
            self.sections
                .push(Self::finalize_section_static(section, table_refs));
        }

        // If there are blocks but no sections, create a default section
        if !self.current_blocks.is_empty() && self.sections.is_empty() {
            self.sections.push(MarkdownSection {
                heading_level: 1,
                heading_text: String::new(),
                content: std::mem::take(&mut self.current_blocks),
                table_refs: std::mem::take(&mut self.table_refs),
            });
        }
    }

    /// Convert a section builder into a MarkdownSection (static version)
    fn finalize_section_static(
        section: SectionBuilder,
        table_refs: Vec<PathBuf>,
    ) -> MarkdownSection {
        MarkdownSection {
            heading_level: section.level,
            heading_text: section.heading_text,
            content: section.blocks,
            table_refs,
        }
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}
