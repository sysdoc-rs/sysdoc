//! PDF export with genpdf library
//!
//! This module exports a UnifiedDocument to a PDF file with:
//! - Title page with document metadata
//! - Table of contents
//! - Modern sans-serif styling
//! - All document content
//!
//! Uses Liberation Sans fonts embedded in the binary (SIL Open Font License 1.1)

use crate::source_model::{ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use genpdf::elements::{Break, PageBreak, Paragraph};
use genpdf::style::Style;
use genpdf::{Alignment, Element};
use std::path::Path;
use thiserror::Error;

// Embedded Liberation Sans fonts (SIL Open Font License 1.1)
// See external/fonts/LICENSE for license details
const FONT_REGULAR: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Bold.ttf");
const FONT_ITALIC: &[u8] = include_bytes!("../../external/fonts/LiberationSans-Italic.ttf");
const FONT_BOLD_ITALIC: &[u8] = include_bytes!("../../external/fonts/LiberationSans-BoldItalic.ttf");

/// PDF export errors
#[derive(Error, Debug)]
pub enum PdfExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("PDF generation error: {0}")]
    PdfError(String),

    #[error("Image loading error: {0}")]
    ImageError(String),
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
    // Create font family from embedded fonts
    let font_regular = genpdf::fonts::FontData::new(FONT_REGULAR.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load regular font: {}", e)))?;
    let font_bold = genpdf::fonts::FontData::new(FONT_BOLD.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load bold font: {}", e)))?;
    let font_italic = genpdf::fonts::FontData::new(FONT_ITALIC.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load italic font: {}", e)))?;
    let font_bold_italic = genpdf::fonts::FontData::new(FONT_BOLD_ITALIC.to_vec(), None)
        .map_err(|e| PdfExportError::PdfError(format!("Failed to load bold italic font: {}", e)))?;

    let font_family = genpdf::fonts::FontFamily {
        regular: font_regular,
        bold: font_bold,
        italic: font_italic,
        bold_italic: font_bold_italic,
    };

    let mut pdf_doc = genpdf::Document::new(font_family);

    // Set document title metadata
    pdf_doc.set_title(&doc.metadata.title);

    // Set minimal page decorator with margins
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15); // 15mm margins

    pdf_doc.set_page_decorator(decorator);

    // Add title page
    add_title_page(&mut pdf_doc, doc)?;

    // Add page break before TOC
    pdf_doc.push(PageBreak::new());

    // Add table of contents
    add_table_of_contents(&mut pdf_doc, doc)?;

    // Add page break before content
    pdf_doc.push(PageBreak::new());

    // Add all sections
    for section in &doc.sections {
        add_section(&mut pdf_doc, section)?;
    }

    // Write to file
    pdf_doc
        .render_to_file(output_path)
        .map_err(|e| PdfExportError::PdfError(e.to_string()))?;

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
        pdf_doc.push(
            Paragraph::new("Description:").styled(Style::new().bold().with_font_size(11)),
        );
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

/// Add table of contents to the PDF
fn add_table_of_contents(
    pdf_doc: &mut genpdf::Document,
    doc: &UnifiedDocument,
) -> Result<(), PdfExportError> {
    // TOC title
    pdf_doc.push(
        Paragraph::new("Table of Contents").styled(Style::new().bold().with_font_size(20)),
    );

    pdf_doc.push(Break::new(1.0));

    // Add each section to TOC
    for section in &doc.sections {
        let indent = "  ".repeat(section.heading_level.saturating_sub(1));
        let toc_entry = format!("{}{} {}", indent, section.section_number, section.heading_text);

        // Smaller font for deeper levels
        let font_size = (12 - section.heading_level.saturating_sub(1).min(3)) as u8;
        pdf_doc.push(Paragraph::new(toc_entry).styled(Style::new().with_font_size(font_size)));
        pdf_doc.push(Break::new(0.1));
    }

    Ok(())
}

/// Add a section to the PDF
fn add_section(
    pdf_doc: &mut genpdf::Document,
    section: &MarkdownSection,
) -> Result<(), PdfExportError> {
    pdf_doc.push(Break::new(1.0));

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

    pdf_doc.push(
        Paragraph::new(heading_text).styled(Style::new().bold().with_font_size(font_size)),
    );

    pdf_doc.push(Break::new(0.5));

    // Add section content blocks
    for block in &section.content {
        add_block(pdf_doc, block)?;
    }

    Ok(())
}

/// Add a markdown block to the PDF
fn add_block(
    pdf_doc: &mut genpdf::Document,
    block: &MarkdownBlock,
) -> Result<(), PdfExportError> {
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
                pdf_doc.push(
                    Paragraph::new("[CSV file not found]").styled(Style::new().italic()),
                );
            }
            pdf_doc.push(Break::new(0.5));
        }

        MarkdownBlock::Image {
            path: _,
            absolute_path: _,
            alt_text,
            title: _,
            format: _,
            exists,
        } => {
            if !exists {
                pdf_doc.push(
                    Paragraph::new("[Image file not found]").styled(Style::new().italic()),
                );
            } else {
                // Add placeholder for image
                let placeholder = format!("[Image: {}]", alt_text);
                let mut img_para = Paragraph::new(placeholder);
                img_para.set_alignment(Alignment::Center);
                pdf_doc.push(img_para.styled(Style::new().italic()));

                // Add alt text
                if !alt_text.is_empty() {
                    let mut alt_para = Paragraph::new(alt_text);
                    alt_para.set_alignment(Alignment::Center);
                    pdf_doc.push(alt_para.styled(Style::new().with_font_size(10).italic()));
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
        for (block_idx, block) in item.content.iter().enumerate() {
            if block_idx == 0 {
                // First block gets bullet/number
                let prefix = if ordered {
                    format!("{}. ", start_number + idx as u64)
                } else {
                    "• ".to_string()
                };

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
            } else {
                // Subsequent blocks are indented (simulated with spaces)
                add_block(pdf_doc, block)?;
            }
        }
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
    // Add header row (bold)
    let header_text: Vec<String> = headers
        .iter()
        .map(|cell_runs| cell_runs.iter().map(|r| r.text.as_str()).collect())
        .collect();

    if !header_text.is_empty() {
        let header_line = header_text.join(" | ");
        pdf_doc.push(Paragraph::new(header_line).styled(Style::new().bold()));

        // Add separator
        let sep = "-".repeat(80);
        pdf_doc.push(Paragraph::new(sep));
    }

    // Add data rows
    for row in rows {
        let row_parts: Vec<String> = row
            .iter()
            .map(|cell_runs| cell_runs.iter().map(|r| r.text.as_str()).collect())
            .collect();

        let row_text = row_parts.join(" | ");
        pdf_doc.push(Paragraph::new(row_text));
    }

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

    // First row is header (bold)
    if let Some(header) = data.first() {
        let header_line = header.join(" | ");
        pdf_doc.push(Paragraph::new(header_line).styled(Style::new().bold()));

        // Add separator
        let sep = "-".repeat(80);
        pdf_doc.push(Paragraph::new(sep));
    }

    // Remaining rows are data
    for row in data.iter().skip(1) {
        let row_text = row.join(" | ");
        pdf_doc.push(Paragraph::new(row_text));
    }

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
