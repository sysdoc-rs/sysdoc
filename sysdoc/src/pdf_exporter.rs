//! PDF export with genpdf library
//!
//! This module exports a UnifiedDocument to a PDF file with:
//! - Title page with document metadata
//! - Table of contents
//! - Modern sans-serif styling
//! - All document content
//!
//! Uses Liberation Sans fonts embedded in the binary (SIL Open Font License 1.1)

use crate::source_model::{ImageFormat, ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use genpdf::elements::{Break, Image, PageBreak, Paragraph, TableLayout};
use genpdf::style::{Color, Style};
use genpdf::{Alignment, Context, Element, Margins, Mm, Position, RenderResult, Size};
use image::GenericImageView;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

// Embedded Liberation Sans fonts (SIL Open Font License 1.1)
// See external/fonts/LICENSE for license details
const FONT_REGULAR: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Bold.ttf");
const FONT_ITALIC: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Italic.ttf");
const FONT_BOLD_ITALIC: &[u8] =
    include_bytes!("../../external/fonts/LiberationSans-BoldItalic.ttf");

/// PDF export errors
#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum PdfExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("PDF generation error: {0}")]
    PdfError(String),

    #[error("Image loading error: {0}")]
    ImageError(String),
}

/// Tracks which page each section appears on using shared state
#[derive(Clone)]
struct SectionPageTracker {
    current_page: Rc<RefCell<usize>>,
    pages: Rc<RefCell<HashMap<String, usize>>>,
}

impl SectionPageTracker {
    fn new() -> Self {
        Self {
            current_page: Rc::new(RefCell::new(0)),
            pages: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn set_current_page(&self, page: usize) {
        *self.current_page.borrow_mut() = page;
    }

    fn mark_section(&self, section_id: String) {
        let page = *self.current_page.borrow();
        self.pages.borrow_mut().insert(section_id, page);
    }

    fn get_pages(&self) -> HashMap<String, usize> {
        self.pages.borrow().clone()
    }
}

/// Custom element that marks a section's page number during rendering
struct SectionMarker {
    section_id: String,
    tracker: SectionPageTracker,
}

impl SectionMarker {
    fn new(section_id: String, tracker: SectionPageTracker) -> Self {
        Self {
            section_id,
            tracker,
        }
    }
}

impl Element for SectionMarker {
    fn render(
        &mut self,
        _context: &Context,
        _area: genpdf::render::Area<'_>,
        _style: Style,
    ) -> Result<RenderResult, genpdf::error::Error> {
        // Mark the section with the current page number
        self.tracker.mark_section(self.section_id.clone());

        // This element doesn't actually render anything visible
        Ok(RenderResult {
            size: Size::new(Mm::from(0.0), Mm::from(0.0)),
            has_more: false,
        })
    }
}

/// Custom page decorator with header and footer support
/// Header: Document ID (left) and Version (right)
/// Footer: Page X of Y (right-aligned)
struct PageNumberFooterDecorator {
    margins: Margins,
    page: usize,
    total_pages: usize,
    document_id: String,
    version: String,
    tracker: Option<SectionPageTracker>,
}

impl PageNumberFooterDecorator {
    fn new(
        total_pages: usize,
        document_id: String,
        version: Option<String>,
        tracker: Option<SectionPageTracker>,
    ) -> Self {
        Self {
            margins: Margins::trbl(15, 15, 15, 15), // top, right, bottom, left in mm
            page: 0,
            total_pages,
            document_id,
            version: version.unwrap_or_else(|| "Draft".to_string()),
            tracker,
        }
    }
}

impl genpdf::PageDecorator for PageNumberFooterDecorator {
    fn decorate_page<'a>(
        &mut self,
        context: &Context,
        mut area: genpdf::render::Area<'a>,
        style: Style,
    ) -> Result<genpdf::render::Area<'a>, genpdf::error::Error> {
        // Increment page counter
        self.page += 1;

        // Update tracker if present
        if let Some(ref tracker) = self.tracker {
            tracker.set_current_page(self.page);
        }

        // Apply margins
        area.add_margins(self.margins);

        // Create header with document ID (left) and version (right)
        let header_height = Mm::from(5.0);
        let header_spacing = Mm::from(3.0);

        // Render document ID on the left
        let mut doc_id_para = Paragraph::new(&self.document_id);
        doc_id_para.set_alignment(Alignment::Left);
        let mut doc_id_element = doc_id_para.styled(Style::new().with_font_size(10));

        let mut header_left_area = area.clone();
        header_left_area.add_offset(Position::new(Mm::from(0.0), Mm::from(0.0)));
        doc_id_element.render(context, header_left_area, style)?;

        // Render version on the right
        let mut version_para = Paragraph::new(&self.version);
        version_para.set_alignment(Alignment::Right);
        let mut version_element = version_para.styled(Style::new().with_font_size(10));

        let mut header_right_area = area.clone();
        header_right_area.add_offset(Position::new(Mm::from(0.0), Mm::from(0.0)));
        version_element.render(context, header_right_area, style)?;

        // Create footer with page number
        let footer_text = format!("Page {} of {}", self.page, self.total_pages);

        // Estimate footer height (one line of 10pt text is approximately 4-5mm)
        let footer_height = Mm::from(5.0);
        let footer_spacing = Mm::from(5.0); // Additional spacing above footer

        // Calculate footer position (bottom of page, right-aligned)
        // Use the full area height before reduction
        let footer_y = area.size().height - footer_height - footer_spacing;

        // Clone the area BEFORE reducing it for the footer
        let mut footer_area = area.clone();
        footer_area.add_offset(Position::new(Mm::from(0.0), footer_y));

        // Create and position the footer at the bottom
        let mut footer = Paragraph::new(&footer_text);
        footer.set_alignment(Alignment::Right);
        let mut footer_element = footer.styled(Style::new().with_font_size(10));

        // Render the footer
        footer_element.render(context, footer_area, style)?;

        // Reduce the available content area to not overlap with header and footer
        // Offset content to start below the header
        area.add_offset(Position::new(Mm::from(0.0), header_height + header_spacing));

        // Reduce height to account for both header and footer
        area.set_size(Size::new(
            area.size().width,
            area.size().height
                - header_height
                - header_spacing
                - footer_height
                - footer_spacing
                - Mm::from(5.0),
        ));

        Ok(area)
    }
}

/// Export a unified document to PDF
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `output_path` - Path where the PDF file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to PDF
/// * `Err(PdfExportError)` - Error during export
pub fn to_pdf(doc: &UnifiedDocument, output_path: &Path) -> Result<(), PdfExportError> {
    // First pass: render to temporary file to count pages
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("sysdoc_temp_{}.pdf", std::process::id()));

    let page_count = {
        let font_family = create_font_family()?;
        let mut pdf_doc = genpdf::Document::new(font_family);
        pdf_doc.set_title(&doc.metadata.title);

        // Use simple decorator for first pass
        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(15);
        pdf_doc.set_page_decorator(decorator);

        // Add all content (no tracking in first pass)
        add_all_content(&mut pdf_doc, doc, None)?;

        // Render to temporary file
        pdf_doc.render_to_file(&temp_file).map_err(|e| {
            PdfExportError::PdfError(format!("Failed to render PDF for page counting: {}", e))
        })?;

        // Count pages by reading the temporary PDF
        count_pdf_pages(&temp_file)?
    };

    // Second pass: render to get section page numbers
    let section_pages = {
        let font_family = create_font_family()?;
        let mut pdf_doc = genpdf::Document::new(font_family);
        pdf_doc.set_title(&doc.metadata.title);

        let section_tracker = SectionPageTracker::new();
        let decorator = PageNumberFooterDecorator::new(
            page_count,
            doc.metadata.document_id.clone(),
            doc.metadata.version.clone(),
            Some(section_tracker.clone()),
        );
        pdf_doc.set_page_decorator(decorator);

        // Track section page numbers
        add_all_content(&mut pdf_doc, doc, Some(section_tracker.clone()))?;

        let temp_file2 = temp_dir.join(format!("sysdoc_temp2_{}.pdf", std::process::id()));
        pdf_doc.render_to_file(&temp_file2).map_err(|e| {
            PdfExportError::PdfError(format!("Failed to render PDF for section tracking: {}", e))
        })?;

        let _ = std::fs::remove_file(&temp_file2);
        section_tracker.get_pages()
    };

    // Clean up first temporary file
    let _ = std::fs::remove_file(&temp_file);

    // Third pass: render final PDF with complete TOC
    let font_family = create_font_family()?;
    let mut pdf_doc = genpdf::Document::new(font_family);

    // Set document title metadata
    pdf_doc.set_title(&doc.metadata.title);

    // Set custom page decorator with headers and footer (no tracker for final pass)
    let decorator = PageNumberFooterDecorator::new(
        page_count,
        doc.metadata.document_id.clone(),
        doc.metadata.version.clone(),
        None,
    );
    pdf_doc.set_page_decorator(decorator);

    // Add title page
    add_title_page(&mut pdf_doc, doc)?;

    // Add page break before TOC
    pdf_doc.push(PageBreak::new());

    // Add table of contents with page numbers
    add_table_of_contents(&mut pdf_doc, doc, Some(&section_pages))?;

    // Add page break before content
    pdf_doc.push(PageBreak::new());

    // Add all sections
    for section in &doc.sections {
        add_section(&mut pdf_doc, section, None)?;
    }

    // Write to final file
    pdf_doc
        .render_to_file(output_path)
        .map_err(|e| PdfExportError::PdfError(e.to_string()))?;

    Ok(())
}

/// Count the number of pages in a PDF file
fn count_pdf_pages(pdf_path: &Path) -> Result<usize, PdfExportError> {
    // Read the PDF file
    let data = std::fs::read(pdf_path)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to read temporary PDF: {}", e)))?;

    // Count occurrences of "/Type /Page" or "/Type/Page" in the PDF
    // This is a simple heuristic that works for most PDFs
    let pdf_str = String::from_utf8_lossy(&data);

    // Try different patterns that might appear in the PDF
    let mut page_count = pdf_str.matches("/Type /Page").count();

    if page_count == 0 {
        page_count = pdf_str.matches("/Type/Page").count();
    }

    if page_count == 0 {
        // Try counting page objects by looking for page dictionaries
        // Look for "/Type /Page\n" or "/Type /Page " patterns
        page_count = pdf_str
            .split("/Type")
            .filter(|s| {
                let trimmed = s.trim_start();
                trimmed.starts_with("/Page") || trimmed.starts_with(" /Page")
            })
            .count();
    }

    if page_count == 0 {
        return Err(PdfExportError::PdfError(
            "Failed to count pages in PDF - no page objects found".to_string(),
        ));
    }

    Ok(page_count)
}

/// Helper function to create font family from embedded fonts
fn create_font_family() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, PdfExportError>
{
    let font_regular = genpdf::fonts::FontData::new(FONT_REGULAR.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load regular font: {}", e)))?;
    let font_bold = genpdf::fonts::FontData::new(FONT_BOLD.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load bold font: {}", e)))?;
    let font_italic = genpdf::fonts::FontData::new(FONT_ITALIC.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load italic font: {}", e)))?;
    let font_bold_italic = genpdf::fonts::FontData::new(FONT_BOLD_ITALIC.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load bold italic font: {}", e)))?;

    Ok(genpdf::fonts::FontFamily {
        regular: font_regular,
        bold: font_bold,
        italic: font_italic,
        bold_italic: font_bold_italic,
    })
}

/// Helper function to add all content to a PDF document
fn add_all_content(
    pdf_doc: &mut genpdf::Document,
    doc: &UnifiedDocument,
    tracker: Option<SectionPageTracker>,
) -> Result<(), PdfExportError> {
    // Add title page
    add_title_page(pdf_doc, doc)?;

    // Add page break before TOC
    pdf_doc.push(PageBreak::new());

    // Add table of contents (without page numbers in tracking pass)
    add_table_of_contents(pdf_doc, doc, None)?;

    // Add page break before content
    pdf_doc.push(PageBreak::new());

    // Add all sections
    for section in &doc.sections {
        add_section(pdf_doc, section, tracker.as_ref())?;
    }

    Ok(())
}

/// Add title page to the PDF
fn add_title_page(
    pdf_doc: &mut genpdf::Document,
    doc: &UnifiedDocument,
) -> Result<(), PdfExportError> {
    // Add vertical space
    pdf_doc.push(Break::new(4.0));

    // Document title - large and bold
    let mut title = Paragraph::new(&doc.metadata.title);
    title.set_alignment(Alignment::Center);
    pdf_doc.push(title.styled(Style::new().bold().with_font_size(24)));

    pdf_doc.push(Break::new(1.5));

    // Subtitle if present
    if let Some(subtitle) = &doc.metadata.subtitle {
        let mut subtitle_para = Paragraph::new(subtitle);
        subtitle_para.set_alignment(Alignment::Center);
        pdf_doc.push(subtitle_para.styled(Style::new().with_font_size(16)));
        pdf_doc.push(Break::new(1.0));
    }

    // Document metadata
    pdf_doc.push(Break::new(3.0));

    add_metadata_row(pdf_doc, "Document ID:", &doc.metadata.document_id);
    add_metadata_row(pdf_doc, "Type:", &doc.metadata.doc_type);
    add_metadata_row(pdf_doc, "Standard:", &doc.metadata.standard);
    add_metadata_row(pdf_doc, "Owner:", &doc.metadata.owner.name);
    add_metadata_row(pdf_doc, "Approver:", &doc.metadata.approver.name);

    if let Some(version) = &doc.metadata.version {
        add_metadata_row(pdf_doc, "Version:", version);
    }

    if let Some(created) = &doc.metadata.created {
        add_metadata_row(pdf_doc, "Created:", created);
    }

    if let Some(modified) = &doc.metadata.modified {
        add_metadata_row(pdf_doc, "Last Modified:", modified);
    }

    // Add description if present
    if let Some(description) = &doc.metadata.description {
        pdf_doc.push(Break::new(1.5));
        pdf_doc.push(Paragraph::new("Description:").styled(Style::new().bold().with_font_size(11)));
        pdf_doc.push(Break::new(0.3));
        pdf_doc.push(Paragraph::new(description).styled(Style::new().with_font_size(11)));
    }

    Ok(())
}

/// Helper to add a metadata row (label and value)
fn add_metadata_row(pdf_doc: &mut genpdf::Document, label: &str, value: &str) {
    let text = format!("{} {}", label, value);
    pdf_doc.push(Paragraph::new(text).styled(Style::new().with_font_size(11)));
    pdf_doc.push(Break::new(0.2));
}

/// Helper function to add a single TOC entry with page number
fn add_toc_entry_with_page_number(
    pdf_doc: &mut genpdf::Document,
    section: &MarkdownSection,
    page: usize,
    toc_font_size: u8,
) -> Result<(), PdfExportError> {
    let indent = "  ".repeat(section.heading_level.saturating_sub(1));

    // Create a two-column table for proper alignment
    // Column widths: 93% for content (with dots), 7% for page number
    let mut table = TableLayout::new(vec![13, 1]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(
        false, false, false,
    ));

    let mut row = table.row();

    // Left cell: section title with dots
    let section_text = format!(
        "{}{} {}",
        indent, section.section_number, section.heading_text
    );

    // Estimate dots: reduce count for longer titles and deeper indents
    let base_dots: usize = 80;
    let indent_reduction = section.heading_level.saturating_sub(1) * 8;
    let title_length_reduction = section_text.len().min(60);
    let dots_count = base_dots
        .saturating_sub(indent_reduction)
        .saturating_sub(title_length_reduction / 2)
        .max(3);

    let dots = " ".to_string() + &".".repeat(dots_count);
    let left_cell = format!("{}{}", section_text, dots);
    row = row.element(Paragraph::new(left_cell).styled(Style::new().with_font_size(toc_font_size)));

    // Right cell: page number (right-aligned)
    let mut page_para = Paragraph::new(format!("{}", page));
    page_para.set_alignment(Alignment::Right);
    row = row.element(page_para.styled(Style::new().with_font_size(toc_font_size)));

    row.push()
        .map_err(|e| PdfExportError::PdfError(format!("Failed to add TOC row: {}", e)))?;

    pdf_doc.push(table);
    Ok(())
}

/// Add table of contents to the PDF
fn add_table_of_contents(
    pdf_doc: &mut genpdf::Document,
    doc: &UnifiedDocument,
    section_pages: Option<&std::collections::HashMap<String, usize>>,
) -> Result<(), PdfExportError> {
    // TOC title
    pdf_doc
        .push(Paragraph::new("Table of Contents").styled(Style::new().bold().with_font_size(20)));

    pdf_doc.push(Break::new(1.0));

    // Use consistent font size for all TOC entries
    let toc_font_size = 11;

    // Add each section to TOC using a table for proper alignment
    for section in &doc.sections {
        let indent = "  ".repeat(section.heading_level.saturating_sub(1));
        let section_id = format!("{} {}", section.section_number, section.heading_text);

        if let Some(pages) = section_pages {
            if let Some(page) = pages.get(&section_id) {
                add_toc_entry_with_page_number(pdf_doc, section, *page, toc_font_size)?;
            } else {
                // No page number available
                let toc_entry = format!(
                    "{}{} {}",
                    indent, section.section_number, section.heading_text
                );
                pdf_doc.push(
                    Paragraph::new(toc_entry).styled(Style::new().with_font_size(toc_font_size)),
                );
            }
        } else {
            // No page numbers at all (used during tracking pass)
            let toc_entry = format!(
                "{}{} {}",
                indent, section.section_number, section.heading_text
            );
            pdf_doc
                .push(Paragraph::new(toc_entry).styled(Style::new().with_font_size(toc_font_size)));
        }

        pdf_doc.push(Break::new(0.1));
    }

    Ok(())
}

/// Add a section to the PDF
fn add_section(
    pdf_doc: &mut genpdf::Document,
    section: &MarkdownSection,
    tracker: Option<&SectionPageTracker>,
) -> Result<(), PdfExportError> {
    pdf_doc.push(Break::new(1.0));

    // Add invisible section marker element that will record page number during rendering
    if let Some(t) = tracker {
        let section_id = format!("{} {}", section.section_number, section.heading_text);
        pdf_doc.push(SectionMarker::new(section_id, t.clone()));
    }

    // Section heading
    let heading_text = format!("{} {}", section.section_number, section.heading_text);

    // Size based on heading level (h1-h6)
    let font_size = match section.heading_level {
        1 => 20,
        2 => 18,
        3 => 16,
        4 => 14,
        5 => 13,
        _ => 12,
    };

    pdf_doc
        .push(Paragraph::new(heading_text).styled(Style::new().bold().with_font_size(font_size)));

    pdf_doc.push(Break::new(0.5));

    // Add section content blocks
    for block in &section.content {
        add_block(pdf_doc, block)?;
    }

    Ok(())
}

/// Add a markdown block to the PDF
fn add_block(pdf_doc: &mut genpdf::Document, block: &MarkdownBlock) -> Result<(), PdfExportError> {
    match block {
        MarkdownBlock::Paragraph(runs) => {
            let para = text_runs_to_paragraph(runs);
            pdf_doc.push(para);
            pdf_doc.push(Break::new(0.4));
        }

        MarkdownBlock::Heading { level, runs } => {
            let font_size = match level {
                1 => 20,
                2 => 18,
                3 => 16,
                4 => 14,
                5 => 13,
                _ => 12,
            };
            let para = text_runs_to_paragraph(runs);
            pdf_doc.push(para.styled(Style::new().bold().with_font_size(font_size)));
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::CodeBlock {
            code,
            language: _,
            fenced: _,
        } => {
            // Code blocks - use smaller font
            pdf_doc.push(Paragraph::new(code.as_str()).styled(Style::new().with_font_size(9)));
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::BlockQuote(blocks) => {
            // Add blockquote blocks
            for inner_block in blocks {
                add_block(pdf_doc, inner_block)?;
            }
            pdf_doc.push(Break::new(0.4));
        }

        MarkdownBlock::List { start, items } => {
            let ordered = start.is_some();
            add_list_items(pdf_doc, items, ordered, start.unwrap_or(1))?;
            pdf_doc.push(Break::new(0.4));
        }

        MarkdownBlock::InlineTable {
            alignments: _,
            headers,
            rows,
        } => {
            add_inline_table(pdf_doc, headers, rows)?;
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::CsvTable {
            path: _,
            absolute_path: _,
            exists,
            data,
        } => {
            if *exists {
                if let Some(csv_data) = data {
                    add_csv_table(pdf_doc, csv_data)?;
                } else {
                    pdf_doc.push(
                        Paragraph::new("[CSV table - data not loaded]")
                            .styled(Style::new().italic()),
                    );
                }
            } else {
                pdf_doc.push(Paragraph::new("[CSV file not found]").styled(Style::new().italic()));
            }
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::Image {
            path,
            absolute_path,
            alt_text,
            title: _,
            format,
            exists,
        } => {
            if !exists {
                pdf_doc.push(
                    Paragraph::new(format!("[Image file not found: {}]", path.display()))
                        .styled(Style::new().italic()),
                );
            } else {
                // Try to load and embed the actual image
                if let Err(e) = load_and_embed_image(pdf_doc, absolute_path, alt_text, format) {
                    // Fall back to placeholder if image loading fails
                    log::warn!("Failed to embed image {}: {}", path.display(), e);
                    let placeholder = format!("[Image: {} (failed to load: {})]", alt_text, e);
                    let mut img_para = Paragraph::new(placeholder);
                    img_para.set_alignment(Alignment::Center);
                    pdf_doc.push(img_para.styled(Style::new().italic()));
                } else if !alt_text.is_empty() {
                    // Image embedded successfully - add caption if alt text is provided
                    let mut caption = Paragraph::new(format!("Figure: {}", alt_text));
                    caption.set_alignment(Alignment::Center);
                    pdf_doc.push(caption.styled(Style::new().with_font_size(10).italic()));
                }
            }
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::Rule => {
            pdf_doc.push(Break::new(0.3));
            pdf_doc.push(Paragraph::new("─".repeat(80)).styled(Style::new().with_font_size(8)));
            pdf_doc.push(Break::new(0.3));
        }

        MarkdownBlock::Html(_) => {
            // Skip HTML blocks in PDF
        }

        MarkdownBlock::IncludedCodeBlock {
            path,
            absolute_path: _,
            language: _,
            content,
            exists,
        } => {
            if !exists {
                pdf_doc.push(
                    Paragraph::new(format!("[Included file not found: {}]", path.display()))
                        .styled(Style::new().italic()),
                );
            } else if let Some(code) = content {
                // Render as code block with smaller font
                pdf_doc.push(Paragraph::new(code.as_str()).styled(Style::new().with_font_size(9)));
            } else {
                pdf_doc.push(
                    Paragraph::new(format!(
                        "[Included file could not be read: {}]",
                        path.display()
                    ))
                    .styled(Style::new().italic()),
                );
            }
            pdf_doc.push(Break::new(0.5));
        }
    }

    Ok(())
}

/// Add list items to the PDF
fn add_list_items(
    pdf_doc: &mut genpdf::Document,
    items: &[ListItem],
    ordered: bool,
    start_number: u64,
) -> Result<(), PdfExportError> {
    for (idx, item) in items.iter().enumerate() {
        let item_number = start_number + idx as u64;
        for (block_idx, block) in item.content.iter().enumerate() {
            if block_idx == 0 {
                // First block gets bullet/number
                let prefix = get_list_prefix(ordered, item_number);
                add_first_list_block(pdf_doc, block, &prefix)?;
            } else {
                // Subsequent blocks are indented (simulated with spaces)
                add_block(pdf_doc, block)?;
            }
        }
    }
    Ok(())
}

/// Get the prefix for a list item (bullet or number)
fn get_list_prefix(ordered: bool, number: u64) -> String {
    if ordered {
        format!("{}. ", number)
    } else {
        "• ".to_string()
    }
}

/// Add the first block of a list item with its prefix
fn add_first_list_block(
    pdf_doc: &mut genpdf::Document,
    block: &MarkdownBlock,
    prefix: &str,
) -> Result<(), PdfExportError> {
    if let MarkdownBlock::Paragraph(runs) = block {
        let mut para = Paragraph::new(prefix);
        // Add text runs
        for run in runs {
            para.push_styled(&run.text, get_text_style(run));
        }
        pdf_doc.push(para);
    } else {
        pdf_doc.push(Paragraph::new(prefix));
        add_block(pdf_doc, block)?;
    }
    Ok(())
}

/// Convert text runs to a paragraph with styling
fn text_runs_to_paragraph(runs: &[TextRun]) -> Paragraph {
    let mut para = Paragraph::new("");

    for run in runs {
        para.push_styled(&run.text, get_text_style(run));
    }

    para
}

/// Get style for a text run
fn get_text_style(run: &TextRun) -> Style {
    let mut style = Style::new();

    if run.bold {
        style = style.bold();
    }
    if run.italic {
        style = style.italic();
    }
    if run.code {
        // Smaller font for inline code
        style = style.with_font_size(10);
    }
    // Note: strikethrough not supported by genpdf

    style
}

/// Add an inline markdown table to the PDF
fn add_inline_table(
    pdf_doc: &mut genpdf::Document,
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
) -> Result<(), PdfExportError> {
    if headers.is_empty() {
        return Ok(());
    }

    // Determine number of columns
    let num_cols = headers.len();

    // Create table layout with equal column widths
    let mut table = TableLayout::new(vec![1; num_cols]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Add header row with bold styling
    let mut header_row = table.row();
    for cell_runs in headers {
        let cell_text: String = cell_runs.iter().map(|r| r.text.as_str()).collect();
        // Add padding using spaces
        let padded_text = format!(" {} ", cell_text);
        let para = Paragraph::new(&padded_text)
            .styled(Style::new().bold().with_color(Color::Rgb(0, 0, 0)));
        header_row = header_row.element(para);
    }
    header_row
        .push()
        .map_err(|e| PdfExportError::PdfError(format!("Failed to add table header row: {}", e)))?;

    // Add data rows
    for row in rows {
        let mut data_row = table.row();

        // Add actual cells
        let actual_cols = row.len().min(num_cols);
        for cell_runs in row.iter().take(actual_cols) {
            let cell_text: String = cell_runs.iter().map(|r| r.text.as_str()).collect();
            // Add padding using spaces
            let padded_text = format!(" {} ", cell_text);
            let para = Paragraph::new(&padded_text);
            data_row = data_row.element(para);
        }

        // Fill missing cells with empty content if row has fewer cells than headers
        for _ in actual_cols..num_cols {
            let para = Paragraph::new(" ");
            data_row = data_row.element(para);
        }

        data_row.push().map_err(|e| {
            PdfExportError::PdfError(format!("Failed to add table data row: {}", e))
        })?;
    }

    pdf_doc.push(table);
    Ok(())
}

/// Add a CSV table to the PDF
fn add_csv_table(
    pdf_doc: &mut genpdf::Document,
    data: &[Vec<String>],
) -> Result<(), PdfExportError> {
    if data.is_empty() {
        return Ok(());
    }

    // Determine number of columns from first row
    let num_cols = data.first().map(|row| row.len()).unwrap_or(0);
    if num_cols == 0 {
        return Ok(());
    }

    // Create table layout with equal column widths
    let mut table = TableLayout::new(vec![1; num_cols]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // First row is header (bold)
    if let Some(header) = data.first() {
        let mut header_row = table.row();
        for cell_text in header {
            // Add padding using spaces
            let padded_text = format!(" {} ", cell_text);
            let para = Paragraph::new(&padded_text)
                .styled(Style::new().bold().with_color(Color::Rgb(0, 0, 0)));
            header_row = header_row.element(para);
        }
        header_row.push().map_err(|e| {
            PdfExportError::PdfError(format!("Failed to add CSV table header row: {}", e))
        })?;
    }

    // Remaining rows are data
    for row in data.iter().skip(1) {
        let mut data_row = table.row();

        // Add actual cells
        let actual_cols = row.len().min(num_cols);
        for cell_text in row.iter().take(actual_cols) {
            // Add padding using spaces
            let padded_text = format!(" {} ", cell_text);
            let para = Paragraph::new(&padded_text);
            data_row = data_row.element(para);
        }

        // Fill missing cells with empty content if row has fewer cells than expected
        for _ in actual_cols..num_cols {
            let para = Paragraph::new(" ");
            data_row = data_row.element(para);
        }

        data_row.push().map_err(|e| {
            PdfExportError::PdfError(format!("Failed to add CSV table data row: {}", e))
        })?;
    }

    pdf_doc.push(table);
    Ok(())
}

/// Load an image file and embed it in the PDF
///
/// # Parameters
/// * `pdf_doc` - The PDF document to add the image to
/// * `image_path` - Absolute path to the image file
/// * `alt_text` - Alternative text for the image (used for error messages)
/// * `format` - Image format (PNG, JPEG, etc.)
///
/// # Returns
/// * `Ok(())` - Image successfully embedded
/// * `Err(PdfExportError)` - Failed to load or embed image
fn load_and_embed_image(
    pdf_doc: &mut genpdf::Document,
    image_path: &Path,
    _alt_text: &str,
    format: &ImageFormat,
) -> Result<(), PdfExportError> {
    // Read the image file
    let image_bytes = std::fs::read(image_path).map_err(|e| {
        PdfExportError::PdfError(format!(
            "Failed to read image file {}: {}",
            image_path.display(),
            e
        ))
    })?;

    // Load image based on format and convert to RGB if needed (genpdf doesn't support alpha channel)
    let image = match format {
        ImageFormat::Jpeg => {
            let dynamic_image = image::load_from_memory_with_format(&image_bytes, image::ImageFormat::Jpeg)
                .map_err(|e| {
                    PdfExportError::PdfError(format!(
                        "Failed to decode JPEG image {}: {}",
                        image_path.display(),
                        e
                    ))
                })?;

            Image::from_dynamic_image(dynamic_image)
                .map_err(|e| {
                    PdfExportError::PdfError(format!(
                        "Failed to create PDF image from JPEG {}: {}",
                        image_path.display(),
                        e
                    ))
                })?
        }
        ImageFormat::Png => {
            let dynamic_image = image::load_from_memory_with_format(&image_bytes, image::ImageFormat::Png)
                .map_err(|e| {
                    PdfExportError::PdfError(format!(
                        "Failed to decode PNG image {}: {}",
                        image_path.display(),
                        e
                    ))
                })?;

            // Convert RGBA to RGB if needed (genpdf doesn't support alpha channel)
            // This composites transparency onto a white background
            let rgb_image = match dynamic_image {
                image::DynamicImage::ImageRgba8(rgba_img) => {
                    // Create white background
                    let (width, height) = rgba_img.dimensions();
                    let mut rgb_img = image::RgbImage::new(width, height);

                    // Composite each pixel onto white background
                    for (x, y, pixel) in rgba_img.enumerate_pixels() {
                        let alpha = pixel[3] as f32 / 255.0;
                        let one_minus_alpha = 1.0 - alpha;

                        // Alpha blend with white background (255, 255, 255)
                        let r = ((pixel[0] as f32 * alpha) + (255.0 * one_minus_alpha)) as u8;
                        let g = ((pixel[1] as f32 * alpha) + (255.0 * one_minus_alpha)) as u8;
                        let b = ((pixel[2] as f32 * alpha) + (255.0 * one_minus_alpha)) as u8;

                        rgb_img.put_pixel(x, y, image::Rgb([r, g, b]));
                    }

                    image::DynamicImage::ImageRgb8(rgb_img)
                }
                image::DynamicImage::ImageLumaA8(luma_a) => {
                    // Convert grayscale with alpha to RGB
                    let (width, height) = luma_a.dimensions();
                    let mut rgb_img = image::RgbImage::new(width, height);

                    for (x, y, pixel) in luma_a.enumerate_pixels() {
                        let gray = pixel[0];
                        let alpha = pixel[1] as f32 / 255.0;
                        let one_minus_alpha = 1.0 - alpha;

                        // Alpha blend grayscale with white background
                        let value = ((gray as f32 * alpha) + (255.0 * one_minus_alpha)) as u8;
                        rgb_img.put_pixel(x, y, image::Rgb([value, value, value]));
                    }

                    image::DynamicImage::ImageRgb8(rgb_img)
                }
                // For other formats, use to_rgb8() which should work fine
                _ => image::DynamicImage::ImageRgb8(dynamic_image.to_rgb8())
            };

            Image::from_dynamic_image(rgb_image)
                .map_err(|e| {
                    PdfExportError::PdfError(format!(
                        "Failed to create PDF image from PNG {}: {}",
                        image_path.display(),
                        e
                    ))
                })?
        }
        ImageFormat::Svg | ImageFormat::DrawIoSvg => {
            return Err(PdfExportError::PdfError(format!(
                "SVG images are not yet supported in PDF export: {}. Please convert to PNG or JPEG.",
                image_path.display()
            )))
        }
        ImageFormat::Other => {
            return Err(PdfExportError::PdfError(format!(
                "Unsupported image format for {}. Only JPEG and PNG are supported.",
                image_path.display()
            )))
        }
    };

    // Calculate appropriate scale based on image dimensions
    // Get the dimensions from the DynamicImage before we convert it
    let (img_width, _img_height) = match format {
        ImageFormat::Jpeg => {
            let dynamic_image =
                image::load_from_memory_with_format(&image_bytes, image::ImageFormat::Jpeg)
                    .map_err(|e| {
                        PdfExportError::PdfError(format!("Failed to get JPEG dimensions: {}", e))
                    })?;
            dynamic_image.dimensions()
        }
        ImageFormat::Png => {
            let dynamic_image =
                image::load_from_memory_with_format(&image_bytes, image::ImageFormat::Png)
                    .map_err(|e| {
                        PdfExportError::PdfError(format!("Failed to get PNG dimensions: {}", e))
                    })?;
            dynamic_image.dimensions()
        }
        _ => (800, 600), // Default for unsupported formats
    };

    // Add the image with center alignment
    let mut img_element = image;
    img_element.set_alignment(Alignment::Center);

    // Calculate scale to fit page width
    // genpdf interprets images at 300 DPI by default: 1 pixel = 1/300 inch = 25.4/300 mm
    // Target width: 170mm (A4 with margins leaves ~180mm, use 170mm for safety)
    let mm_per_pixel = 25.4 / 300.0; // 0.0847 mm per pixel at 300 DPI
    let image_width_mm = img_width as f64 * mm_per_pixel;
    let target_width_mm = 170.0;

    // Calculate scale factor to fit within target width
    let scale_factor = if image_width_mm > target_width_mm {
        // Scale down large images
        target_width_mm / image_width_mm
    } else {
        // Scale up small images, but cap at 2x to avoid pixelation
        (target_width_mm / image_width_mm).min(2.0)
    };

    img_element.set_scale(genpdf::Scale::new(scale_factor, scale_factor));

    pdf_doc.push(img_element);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified_document::{DocumentMetadata, Person};
    use std::path::PathBuf;

    #[test]
    fn test_pdf_export_structure() {
        // Test that the structures are correct (actual export requires fonts)
        let metadata = DocumentMetadata {
            system_id: Some("TEST-001".to_string()),
            document_id: "PDF-TEST-001".to_string(),
            title: "Test PDF Document".to_string(),
            subtitle: Some("A test document".to_string()),
            description: Some("Testing PDF export".to_string()),
            doc_type: "TEST".to_string(),
            standard: "TEST-STANDARD".to_string(),
            template: "test".to_string(),
            owner: Person {
                name: "Test Owner".to_string(),
                email: "owner@test.com".to_string(),
            },
            approver: Person {
                name: "Test Approver".to_string(),
                email: "approver@test.com".to_string(),
            },
            version: Some("1.0.0".to_string()),
            created: Some("2024-01-01".to_string()),
            modified: Some("2024-01-02".to_string()),
        };

        let doc = UnifiedDocument {
            metadata,
            sections: vec![],
            tables: vec![],
            root: PathBuf::from("."),
        };

        // Cannot test actual PDF generation without font files
        // Just verify the structure compiles
        assert_eq!(doc.sections.len(), 0);
    }
}
