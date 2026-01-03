//! DOCX export using the docx-rs library
//!
//! This module handles exporting unified documents to Microsoft Word (.docx) format
//! using the `docx-rs` crate. Unlike docx-rust, this creates documents from scratch
//! without requiring a template file.

use crate::source_model::{Alignment, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use docx_rs::{
    AlignmentType, Docx, Paragraph, Pic, Run, RunFonts, Style, StyleType, Table, TableCell,
    TableRow, WidthType,
};
use std::path::Path;

/// EMUs (English Metric Units) per inch - Word uses this for measurements
const EMUS_PER_INCH: u32 = 914400;

/// Default DPI for images without embedded DPI information
const DEFAULT_IMAGE_DPI: f64 = 96.0;

/// Maximum image width in inches (to fit on a standard page with margins)
const MAX_IMAGE_WIDTH_INCHES: f64 = 6.5;

/// Export to Microsoft Word (.docx) using docx-rs
///
/// This exporter creates documents from scratch with built-in heading styles.
/// Unlike the docx-rust exporter, it does not require a template file.
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `output_path` - Path where the .docx file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to DOCX format
/// * `Err(ExportError)` - Error during export
pub fn to_docx(doc: &UnifiedDocument, output_path: &Path) -> Result<(), ExportError> {
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(ExportError::IoError)?;
    }

    log::info!(
        "Creating DOCX with docx-rs: {} sections",
        doc.sections.len()
    );

    // Create a new document with heading styles
    let mut docx = Docx::new();

    // Add heading styles
    docx = add_heading_styles(docx);

    // Add document sections
    for section in &doc.sections {
        docx = append_section(docx, section)?;
    }

    // Write the document
    log::info!("Writing DOCX to: {}", output_path.display());
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(ExportError::IoError)?;
    }
    let file = std::fs::File::create(output_path).map_err(ExportError::IoError)?;
    docx.build()
        .pack(file)
        .map_err(|e| ExportError::FormatError(format!("Failed to write DOCX: {}", e)))?;

    log::info!(
        "Successfully wrote DOCX with {} sections",
        doc.sections.len()
    );
    Ok(())
}

/// Add heading styles to the document
fn add_heading_styles(mut docx: Docx) -> Docx {
    // Define heading styles with progressively smaller sizes
    let heading_sizes = [
        ("Heading1", 32), // 16pt
        ("Heading2", 28), // 14pt
        ("Heading3", 26), // 13pt
        ("Heading4", 24), // 12pt
        ("Heading5", 22), // 11pt
        ("Heading6", 20), // 10pt
        ("Heading7", 20),
        ("Heading8", 20),
        ("Heading9", 20),
    ];

    for (style_id, size) in heading_sizes {
        let style = Style::new(style_id, StyleType::Paragraph)
            .name(style_id)
            .bold()
            .size(size * 2); // docx-rs uses half-points
        docx = docx.add_style(style);
    }

    docx
}

/// Get the heading style ID for a given heading level
fn heading_style_id(level: usize) -> &'static str {
    match level {
        1 => "Heading1",
        2 => "Heading2",
        3 => "Heading3",
        4 => "Heading4",
        5 => "Heading5",
        6 => "Heading6",
        7 => "Heading7",
        8 => "Heading8",
        9 => "Heading9",
        _ if level < 1 => "Heading1",
        _ => "Heading9",
    }
}

/// Append a document section to the docx
fn append_section(mut docx: Docx, section: &MarkdownSection) -> Result<Docx, ExportError> {
    // Create heading text
    let heading_text = format!("{} {}", section.section_number, section.heading_text);

    // Calculate heading level from section number depth
    let heading_level = section.section_number.depth() + 1;
    let style_id = heading_style_id(heading_level);

    // Create heading paragraph
    let heading_para = Paragraph::new()
        .style(style_id)
        .add_run(Run::new().add_text(&heading_text));
    docx = docx.add_paragraph(heading_para);

    // Append content blocks
    for block in &section.content {
        docx = append_block(docx, block)?;
    }

    Ok(docx)
}

/// Append a MarkdownBlock to the docx document
fn append_block(mut docx: Docx, block: &MarkdownBlock) -> Result<Docx, ExportError> {
    match block {
        MarkdownBlock::Paragraph(runs) => {
            let para = create_paragraph(runs);
            docx = docx.add_paragraph(para);
        }
        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            exists,
            ..
        } => {
            if *exists {
                match create_image_paragraph(absolute_path, alt_text) {
                    Ok(para) => docx = docx.add_paragraph(para),
                    Err(e) => {
                        log::warn!("Failed to add image {}: {}", absolute_path.display(), e);
                        let para = Paragraph::new().add_run(
                            Run::new()
                                .add_text(format!("[Image error: {}]", absolute_path.display())),
                        );
                        docx = docx.add_paragraph(para);
                    }
                }
            } else {
                let para = Paragraph::new().add_run(
                    Run::new().add_text(format!("[Missing image: {}]", absolute_path.display())),
                );
                docx = docx.add_paragraph(para);
            }
        }
        MarkdownBlock::CsvTable {
            path, exists, data, ..
        } => {
            docx = append_csv_table(docx, path, *exists, data);
        }
        MarkdownBlock::InlineTable {
            alignments,
            headers,
            rows,
        } => {
            let table = create_inline_table(alignments, headers, rows);
            docx = docx.add_table(table);
        }
        _ => {
            // For unhandled block types, add a placeholder
            let para = Paragraph::new().add_run(
                Run::new().add_text(format!("[{} not yet implemented]", block_type_name(block))),
            );
            docx = docx.add_paragraph(para);
        }
    }

    Ok(docx)
}

/// Get a human-readable name for a MarkdownBlock type
fn block_type_name(block: &MarkdownBlock) -> &'static str {
    match block {
        MarkdownBlock::Heading { .. } => "Heading",
        MarkdownBlock::Paragraph(_) => "Paragraph",
        MarkdownBlock::Image { .. } => "Image",
        MarkdownBlock::CodeBlock { .. } => "CodeBlock",
        MarkdownBlock::BlockQuote(_) => "BlockQuote",
        MarkdownBlock::List { .. } => "List",
        MarkdownBlock::InlineTable { .. } => "InlineTable",
        MarkdownBlock::CsvTable { .. } => "CsvTable",
        MarkdownBlock::Rule => "Rule",
        MarkdownBlock::Html(_) => "Html",
        MarkdownBlock::IncludedCodeBlock { .. } => "IncludedCodeBlock",
    }
}

/// Create a docx Paragraph from a vector of TextRuns
fn create_paragraph(runs: &[TextRun]) -> Paragraph {
    let mut para = Paragraph::new();
    for text_run in runs {
        let run = create_run(text_run);
        para = para.add_run(run);
    }
    para
}

/// Create a docx Run from a TextRun with appropriate formatting
fn create_run(text_run: &TextRun) -> Run {
    let mut run = Run::new().add_text(&text_run.text);

    if text_run.bold {
        run = run.bold();
    }
    if text_run.italic {
        run = run.italic();
    }
    if text_run.strikethrough {
        run = run.strike();
    }
    if text_run.code {
        run = run.fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas"));
    }

    run
}

/// Create an image paragraph
fn create_image_paragraph(
    absolute_path: &std::path::PathBuf,
    _alt_text: &str,
) -> Result<Paragraph, ExportError> {
    let bytes = std::fs::read(absolute_path).map_err(ExportError::IoError)?;

    // Get image dimensions
    let (width_emu, height_emu) = match imagesize::blob_size(&bytes) {
        Ok(size) if size.width > 0 && size.height > 0 => {
            // Calculate natural size in inches based on pixel dimensions
            let natural_width_inches = size.width as f64 / DEFAULT_IMAGE_DPI;
            let aspect_ratio = size.height as f64 / size.width as f64;

            // Scale to fit within max width while preserving aspect ratio
            let final_width_inches = natural_width_inches.min(MAX_IMAGE_WIDTH_INCHES);
            let final_height_inches = final_width_inches * aspect_ratio;

            let width = (final_width_inches * EMUS_PER_INCH as f64) as u32;
            let height = (final_height_inches * EMUS_PER_INCH as f64) as u32;
            (width, height)
        }
        _ => {
            // Fallback to default 6x4 inches if dimensions unknown
            let width = (6.0 * EMUS_PER_INCH as f64) as u32;
            let height = (4.0 * EMUS_PER_INCH as f64) as u32;
            (width, height)
        }
    };

    // Create the image
    let pic = Pic::new(&bytes).size(width_emu, height_emu);

    // Create centered paragraph with image
    let para = Paragraph::new()
        .align(AlignmentType::Center)
        .add_run(Run::new().add_image(pic));

    Ok(para)
}

/// Append a CSV table to the document
fn append_csv_table(
    docx: Docx,
    path: &Path,
    exists: bool,
    data: &Option<Vec<Vec<String>>>,
) -> Docx {
    if !exists {
        let para = Paragraph::new()
            .add_run(Run::new().add_text(format!("[Missing CSV file: {}]", path.display())));
        return docx.add_paragraph(para);
    }

    let Some(csv_data) = data else {
        let para = Paragraph::new()
            .add_run(Run::new().add_text(format!("[Failed to load CSV: {}]", path.display())));
        return docx.add_paragraph(para);
    };

    if csv_data.is_empty() {
        let para = Paragraph::new()
            .add_run(Run::new().add_text(format!("[Empty CSV file: {}]", path.display())));
        return docx.add_paragraph(para);
    }

    let table = create_csv_table(csv_data);
    docx.add_table(table)
}

/// Create a DOCX table from CSV data
fn create_csv_table(data: &[Vec<String>]) -> Table {
    let mut rows = Vec::new();

    for (row_idx, row_data) in data.iter().enumerate() {
        let is_header = row_idx == 0;
        let table_row = create_table_row_from_strings(row_data, is_header);
        rows.push(table_row);
    }

    Table::new(rows)
}

/// Create a table row from a vector of cell strings
fn create_table_row_from_strings(cells: &[String], is_header: bool) -> TableRow {
    let table_cells: Vec<TableCell> = cells
        .iter()
        .map(|cell_text| create_table_cell_from_string(cell_text, is_header))
        .collect();

    TableRow::new(table_cells)
}

/// Create a table cell with text content
fn create_table_cell_from_string(text: &str, bold: bool) -> TableCell {
    let mut run = Run::new().add_text(text);
    if bold {
        run = run.bold();
    }

    let para = Paragraph::new().add_run(run);
    TableCell::new().add_paragraph(para)
}

/// Convert Alignment to AlignmentType for paragraph formatting
fn alignment_to_docx_alignment(alignment: Alignment) -> AlignmentType {
    match alignment {
        Alignment::Left | Alignment::None => AlignmentType::Left,
        Alignment::Center => AlignmentType::Center,
        Alignment::Right => AlignmentType::Right,
    }
}

/// Create a DOCX table from inline markdown table data
fn create_inline_table(
    alignments: &[Alignment],
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
) -> Table {
    let mut table_rows = Vec::new();

    // Add header row (with bold formatting)
    let header_row = create_inline_table_row(headers, alignments, true);
    table_rows.push(header_row);

    // Add data rows
    for row_data in rows {
        let data_row = create_inline_table_row(row_data, alignments, false);
        table_rows.push(data_row);
    }

    Table::new(table_rows)
}

/// Create a table row from cells containing formatted text runs
fn create_inline_table_row(
    cells: &[Vec<TextRun>],
    alignments: &[Alignment],
    is_header: bool,
) -> TableRow {
    let table_cells: Vec<TableCell> = cells
        .iter()
        .enumerate()
        .map(|(col_idx, cell_runs)| {
            let alignment = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
            create_inline_table_cell(cell_runs, alignment, is_header)
        })
        .collect();

    TableRow::new(table_cells)
}

/// Create a table cell from formatted text runs
fn create_inline_table_cell(runs: &[TextRun], alignment: Alignment, make_bold: bool) -> TableCell {
    let docx_alignment = alignment_to_docx_alignment(alignment);
    let mut para = Paragraph::new().align(docx_alignment);

    if runs.is_empty() {
        // Empty cells still need at least one run
        let mut run = Run::new().add_text("");
        if make_bold {
            run = run.bold();
        }
        para = para.add_run(run);
    } else {
        for text_run in runs {
            let run = create_text_run(text_run, make_bold);
            para = para.add_run(run);
        }
    }

    TableCell::new()
        .width(2000, WidthType::Dxa)
        .add_paragraph(para)
}

/// Create a DOCX Run from a TextRun with formatting
fn create_text_run(text_run: &TextRun, make_bold: bool) -> Run {
    let mut run = Run::new().add_text(&text_run.text);

    // Apply bold if this is a header row OR if the text run itself is bold
    if make_bold || text_run.bold {
        run = run.bold();
    }
    if text_run.italic {
        run = run.italic();
    }
    if text_run.strikethrough {
        run = run.strike();
    }
    if text_run.code {
        run = run.fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas"));
    }

    run
}

/// Export errors
#[derive(Debug)]
pub enum ExportError {
    IoError(std::io::Error),
    FormatError(String),
    NotImplemented(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::IoError(e) => write!(f, "IO error: {}", e),
            ExportError::FormatError(msg) => write!(f, "Format error: {}", msg),
            ExportError::NotImplemented(feature) => {
                write!(f, "Not implemented: {}", feature)
            }
        }
    }
}

impl std::error::Error for ExportError {}
