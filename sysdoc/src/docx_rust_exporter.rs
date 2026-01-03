//! DOCX export using the docx-rust library
//!
//! This module handles exporting unified documents to Microsoft Word (.docx) format
//! using the `docx-rust` crate. It works by loading a template DOCX file and
//! appending content sections to it.

use crate::source_model::{Alignment, ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use docx_rust::{
    document::{
        Blip, BlipFill, CNvPicPr, CNvPr, DocPr, Drawing, Ext, Extent, FillRect, Graphic,
        GraphicData, Inline, NvPicPr, Paragraph, Picture, PrstGeom, Run, SpPr, Stretch, Table,
        TableCell, TableGrid, TableRow, TextSpace, Xfrm,
    },
    formatting::{CharacterProperty, Fonts, Indent, JustificationVal, ParagraphProperty},
    media::MediaType,
    rels::Relationship,
    Docx, DocxFile,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get the Word built-in heading style ID for a given heading level
///
/// Maps heading levels 1-9 to Word's built-in style IDs "Heading1" through "Heading9".
/// Levels outside this range are clamped to the valid range.
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

/// Pre-loaded image data with metadata for DOCX export
struct ImageData {
    /// The image bytes
    bytes: Vec<u8>,
    /// Media path within the docx (e.g., "media/image1.png")
    media_path: String,
    /// Relationship ID (e.g., "rId10")
    rel_id: String,
    /// Image width in pixels (if available)
    width_px: Option<usize>,
    /// Image height in pixels (if available)
    height_px: Option<usize>,
}

/// Image lookup info: (relationship_id, image_index, width_px, height_px)
type ImageLookupInfo<'a> = (&'a str, isize, Option<usize>, Option<usize>);

/// Collect and load all images from document sections
fn collect_images(sections: &[MarkdownSection]) -> HashMap<PathBuf, ImageData> {
    let mut images = HashMap::new();
    let mut image_counter = 1;
    // Start relationship IDs high to avoid conflicts with template
    let mut rel_id_counter = 100;

    let all_blocks = sections.iter().flat_map(|s| &s.content);
    for block in all_blocks {
        if let Some(image_data) = try_load_image(block, &images, image_counter, rel_id_counter) {
            images.insert(image_data.0, image_data.1);
            image_counter += 1;
            rel_id_counter += 1;
        }
    }

    images
}

/// Try to load an image from a block, returning None if it's not an image or can't be loaded
fn try_load_image(
    block: &MarkdownBlock,
    existing: &HashMap<PathBuf, ImageData>,
    image_counter: usize,
    rel_id_counter: usize,
) -> Option<(PathBuf, ImageData)> {
    let MarkdownBlock::Image {
        absolute_path,
        exists,
        ..
    } = block
    else {
        return None;
    };

    if !*exists || existing.contains_key(absolute_path) {
        return None;
    }

    let bytes = std::fs::read(absolute_path).ok()?;

    let extension = absolute_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let media_path = format!("media/image{}.{}", image_counter, extension);
    let rel_id = format!("rId{}", rel_id_counter);

    let (width_px, height_px) = imagesize::blob_size(&bytes)
        .map(|size| (Some(size.width), Some(size.height)))
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not read dimensions for {}: {}",
                absolute_path.display(),
                e
            );
            (None, None)
        });

    Some((
        absolute_path.clone(),
        ImageData {
            bytes,
            media_path,
            rel_id,
            width_px,
            height_px,
        },
    ))
}

/// EMUs (English Metric Units) per inch - Word uses this for measurements
const EMUS_PER_INCH: i64 = 914400;

/// Default image width in inches (6 inches fits well on a page with margins)
const DEFAULT_IMAGE_WIDTH_INCHES: f64 = 6.0;

/// Default DPI for images without embedded DPI information
const DEFAULT_IMAGE_DPI: f64 = 96.0;

/// Maximum image width in inches (to fit on a standard page with margins)
const MAX_IMAGE_WIDTH_INCHES: f64 = 6.5;

/// Create a Drawing element for an inline image
///
/// If width_px and height_px are provided, the image dimensions are calculated
/// to preserve the original aspect ratio while fitting within the page width.
fn create_image_drawing(
    rel_id: &str,
    image_id: isize,
    alt_text: &str,
    width_px: Option<usize>,
    height_px: Option<usize>,
) -> Drawing<'static> {
    // Calculate dimensions preserving aspect ratio
    let (width_emu, height_emu) = match (width_px, height_px) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            // Calculate natural size in inches based on pixel dimensions
            let natural_width_inches = w as f64 / DEFAULT_IMAGE_DPI;
            let aspect_ratio = h as f64 / w as f64;

            // Scale to fit within max width while preserving aspect ratio
            let final_width_inches = natural_width_inches.min(MAX_IMAGE_WIDTH_INCHES);
            let final_height_inches = final_width_inches * aspect_ratio;

            let width = (final_width_inches * EMUS_PER_INCH as f64) as u64;
            let height = (final_height_inches * EMUS_PER_INCH as f64) as u64;
            (width, height)
        }
        _ => {
            // Fallback to default 6x4 inches if dimensions unknown
            let width = (DEFAULT_IMAGE_WIDTH_INCHES * EMUS_PER_INCH as f64) as u64;
            let height = (DEFAULT_IMAGE_WIDTH_INCHES * 0.667 * EMUS_PER_INCH as f64) as u64;
            (width, height)
        }
    };

    Drawing {
        anchor: None,
        inline: Some(Inline {
            dist_t: Some(0),
            dist_b: Some(0),
            dist_l: Some(0),
            dist_r: Some(0),
            extent: Some(Extent {
                cx: width_emu,
                cy: height_emu,
            }),
            doc_property: DocPr {
                id: Some(image_id),
                name: Some(format!("Picture {}", image_id).into()),
                descr: Some(alt_text.to_string().into()),
            },
            graphic: Some(Graphic {
                a: "http://schemas.openxmlformats.org/drawingml/2006/main".into(),
                data: GraphicData {
                    uri: "http://schemas.openxmlformats.org/drawingml/2006/picture".into(),
                    children: vec![Picture {
                        a: "http://schemas.openxmlformats.org/drawingml/2006/picture".into(),
                        nv_pic_pr: NvPicPr {
                            c_nv_pr: Some(CNvPr {
                                id: Some(0),
                                name: Some(format!("Picture {}", image_id).into()),
                                descr: Some(alt_text.to_string().into()),
                            }),
                            c_nv_pic_pr: Some(CNvPicPr {}),
                        },
                        fill: BlipFill {
                            blip: Blip {
                                embed: rel_id.to_string().into(),
                                cstate: None,
                            },
                            stretch: Some(Stretch {
                                fill_rect: Some(FillRect {}),
                            }),
                        },
                        sp_pr: SpPr {
                            xfrm: Some(Xfrm {
                                offset: None,
                                ext: Some(Ext {
                                    cx: Some(width_emu as isize),
                                    cy: Some(height_emu as isize),
                                }),
                            }),
                            prst_geom: Some(PrstGeom {
                                prst: Some("rect".into()),
                                av_lst: None,
                            }),
                        },
                    }],
                },
            }),
            // Set remaining fields to None/defaults
            simple_pos_attr: None,
            relative_height: None,
            behind_doc: None,
            locked: None,
            layout_in_cell: None,
            allow_overlap: None,
            simple_pos: None,
            position_horizontal: None,
            position_vertical: None,
        }),
    }
}

/// Export to Microsoft Word (.docx)
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `template_path` - Path to a .docx template file containing style definitions
/// * `output_path` - Path where the .docx file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to DOCX format
/// * `Err(ExportError)` - Error during export (IO, format, or docx-rust errors)
///
/// # Notes
/// A template is required because it provides the style definitions (Heading1, Heading2, etc.)
/// that the exported document references. Without a template, the styles would not render
/// correctly in Word.
pub fn to_docx(
    doc: &UnifiedDocument,
    template_path: &Path,
    output_path: &Path,
) -> Result<(), ExportError> {
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(ExportError::IoError)?;
    }

    // Pre-collect all heading strings so they have a stable lifetime
    let heading_strings = collect_all_headings(&doc.sections);

    // Pre-collect all images so they have a stable lifetime
    let images = collect_images(&doc.sections);
    log::info!("Collected {} images for embedding", images.len());

    // Read the template file - required for style definitions
    log::info!("Reading template from: {}", template_path.display());
    let docx_file = DocxFile::from_file(template_path)
        .map_err(|e| ExportError::FormatError(format!("Failed to read template: {}", e)))?;
    let mut docx = docx_file
        .parse()
        .map_err(|e| ExportError::FormatError(format!("Failed to parse template: {}", e)))?;

    // Add images to docx media and relationships
    for (path, image_data) in &images {
        log::debug!(
            "Adding image: {} -> {} ({})",
            path.display(),
            image_data.media_path,
            image_data.rel_id
        );

        // Add to media (the bytes reference the pre-collected images HashMap)
        docx.media.insert(
            image_data.media_path.clone(),
            (MediaType::Image, &image_data.bytes),
        );

        // Add relationship with specific ID
        docx.document_rels
            .get_or_insert(Default::default())
            .relationships
            .push(Relationship {
                id: image_data.rel_id.clone().into(),
                target: image_data.media_path.clone().into(),
                ty: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                    .into(),
                target_mode: None,
            });
    }

    // Create a lookup map from absolute path to (rel_id, image_index, width, height) for use in append_block
    let image_lookup: HashMap<&PathBuf, ImageLookupInfo<'_>> = images
        .iter()
        .enumerate()
        .map(|(idx, (path, data))| {
            (
                path,
                (
                    data.rel_id.as_str(),
                    idx as isize + 1,
                    data.width_px,
                    data.height_px,
                ),
            )
        })
        .collect();

    // Add document sections using pre-collected headings
    let mut heading_index = 0;
    for section in &doc.sections {
        append_section(
            &mut docx,
            section,
            &heading_strings,
            &mut heading_index,
            &image_lookup,
        )?;
    }

    // Write the document
    log::info!("Writing DOCX to: {}", output_path.display());
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(ExportError::IoError)?;
    }
    docx.write_file(output_path)
        .map_err(|e| ExportError::FormatError(format!("Failed to write DOCX: {}", e)))?;

    log::info!(
        "Successfully wrote DOCX with {} sections",
        doc.sections.len()
    );
    Ok(())
}

/// Collect all heading strings beforehand so they have a stable lifetime
fn collect_all_headings(sections: &[MarkdownSection]) -> Vec<String> {
    sections
        .iter()
        .map(|section| format!("{} {}", section.section_number, section.heading_text))
        .collect()
}

/// Append a document section to the docx
fn append_section<'a>(
    docx: &mut Docx<'a>,
    section: &MarkdownSection,
    heading_strings: &'a [String],
    heading_index: &mut usize,
    image_lookup: &HashMap<&PathBuf, ImageLookupInfo<'_>>,
) -> Result<(), ExportError> {
    // Get the pre-generated heading string
    let heading_ref = heading_strings[*heading_index].as_str();
    *heading_index += 1;

    // Calculate heading level from section number depth
    // depth 0 (e.g., "1") -> Heading1, depth 1 (e.g., "1.1") -> Heading2, etc.
    let heading_level = section.section_number.depth() + 1;
    let style_id = heading_style_id(heading_level);
    let para = Paragraph::default()
        .property(ParagraphProperty::default().style_id(style_id))
        .push_text(heading_ref);
    docx.document.push(para);

    // Append content blocks
    for block in &section.content {
        append_block(docx, block, image_lookup);
    }

    Ok(())
}

/// Append a CSV table to the document
///
/// Handles the various states a CSV table reference can be in:
/// - File doesn't exist: shows error placeholder
/// - Data is None (failed to load): shows error placeholder
/// - Data is empty: shows warning placeholder
/// - Data is valid: creates and appends a DOCX table
fn append_csv_table(
    docx: &mut Docx<'_>,
    path: &Path,
    exists: bool,
    data: &Option<Vec<Vec<String>>>,
) {
    if !exists {
        let para =
            Paragraph::default().push_text(format!("[Missing CSV file: {}]", path.display()));
        docx.document.push(para);
        return;
    }

    let Some(csv_data) = data else {
        let para =
            Paragraph::default().push_text(format!("[Failed to load CSV: {}]", path.display()));
        docx.document.push(para);
        return;
    };

    if csv_data.is_empty() {
        let para = Paragraph::default().push_text(format!("[Empty CSV file: {}]", path.display()));
        docx.document.push(para);
        return;
    }

    let table = create_csv_table(csv_data);
    docx.document.push(table);
}

/// Create a paragraph for an image block
fn create_image_paragraph(
    absolute_path: &PathBuf,
    alt_text: &str,
    exists: bool,
    image_lookup: &HashMap<&PathBuf, ImageLookupInfo<'_>>,
) -> Paragraph<'static> {
    if !exists {
        return Paragraph::default()
            .push_text(format!("[Missing image: {}]", absolute_path.display()));
    }

    let Some((rel_id, image_id, width_px, height_px)) = image_lookup.get(absolute_path) else {
        return Paragraph::default()
            .push_text(format!("[Image not found: {}]", absolute_path.display()));
    };

    let drawing = create_image_drawing(rel_id, *image_id, alt_text, *width_px, *height_px);
    let run = Run::default().push(drawing);
    Paragraph::default()
        .property(ParagraphProperty::default().justification(JustificationVal::Center))
        .push(run)
}

/// Append a MarkdownBlock to the docx document
///
/// Converts markdown block elements to their docx equivalents.
/// Currently supports:
/// - Paragraph: Converted to docx paragraph with formatted text runs
/// - Image: Converted to inline drawing with embedded image (centered, aspect ratio preserved)
fn append_block(
    docx: &mut Docx<'_>,
    block: &MarkdownBlock,
    image_lookup: &HashMap<&PathBuf, ImageLookupInfo<'_>>,
) {
    match block {
        MarkdownBlock::Paragraph(runs) => {
            let para = create_paragraph(runs);
            docx.document.push(para);
        }
        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            exists,
            ..
        } => {
            let para = create_image_paragraph(absolute_path, alt_text, *exists, image_lookup);
            docx.document.push(para);
        }
        MarkdownBlock::CsvTable {
            path, exists, data, ..
        } => {
            append_csv_table(docx, path, *exists, data);
        }
        MarkdownBlock::InlineTable {
            alignments,
            headers,
            rows,
        } => {
            let table = create_inline_table(alignments, headers, rows);
            docx.document.push(table);
        }
        MarkdownBlock::List { start, items } => {
            append_list(docx, start, items, 0, image_lookup);
        }
        MarkdownBlock::CodeBlock { code, .. } => {
            // Render code blocks with monospace font
            let para = create_code_block_paragraph(code);
            docx.document.push(para);
        }
        MarkdownBlock::IncludedCodeBlock {
            path,
            content,
            exists,
            ..
        } => {
            // Render included code blocks with monospace font
            if !*exists {
                let para = Paragraph::default()
                    .push_text(format!("[Missing include file: {}]", path.display()));
                docx.document.push(para);
            } else if let Some(code) = content {
                let para = create_code_block_paragraph(code);
                docx.document.push(para);
            } else {
                let para = Paragraph::default()
                    .push_text(format!("[Failed to read include file: {}]", path.display()));
                docx.document.push(para);
            }
        }
        MarkdownBlock::BlockQuote(blocks) => {
            // Render block quotes with indentation
            for inner_block in blocks {
                append_block_with_indent(docx, inner_block, 1, image_lookup);
            }
        }
        MarkdownBlock::Rule => {
            // Render horizontal rule as a separator line
            let para = Paragraph::default().push_text("─".repeat(50));
            docx.document.push(para);
        }
        _ => {
            // For unhandled block types, add a placeholder
            let para = Paragraph::default().push_text(format!(
                "[{:?} not yet implemented]",
                block_type_name(block)
            ));
            docx.document.push(para);
        }
    }
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
fn create_paragraph(runs: &[TextRun]) -> Paragraph<'static> {
    let mut para = Paragraph::default();
    for text_run in runs {
        let run = create_run(text_run);
        para = para.push(run);
    }
    para
}

/// Create a docx Run from a TextRun with appropriate formatting
///
/// Uses direct formatting via CharacterProperty fields:
/// - bold => bold(true)
/// - italic => italics(true)
/// - strikethrough => strike(true)
/// - code => monospace font (Consolas)
fn create_run(text_run: &TextRun) -> Run<'static> {
    let text = text_run.text.clone();
    let mut prop = CharacterProperty::default();

    if text_run.bold {
        prop = prop.bold(true);
    }
    if text_run.italic {
        prop = prop.italics(true);
    }
    if text_run.strikethrough {
        prop = prop.strike(true);
    }
    if text_run.code {
        prop = prop.fonts(Fonts::default().ascii("Consolas").h_ansi("Consolas"));
    }

    Run::default()
        .property(prop)
        .push_text((text, TextSpace::Preserve))
}

/// Create a DOCX table from CSV data
///
/// Converts a 2D vector of strings (where the first row is headers) into a DOCX table.
/// Headers are formatted in bold. Column widths are distributed evenly.
///
/// # Parameters
/// * `data` - 2D vector where first row is headers, remaining rows are data
///
/// # Returns
/// * `Table` - A formatted DOCX table ready to be inserted into the document
fn create_csv_table(data: &[Vec<String>]) -> Table<'static> {
    let mut table = Table::default();

    // Determine number of columns from the first row (or 0 if empty)
    let num_cols = data.first().map(|row| row.len()).unwrap_or(0);

    // Create table grid with equal column widths
    // Using a reasonable width value (2000 twips per column)
    let mut grid = TableGrid::default();
    for _ in 0..num_cols {
        grid = grid.push_column(2000);
    }
    table.grids = grid;

    // Process rows
    for (row_idx, row_data) in data.iter().enumerate() {
        let is_header = row_idx == 0;
        let table_row = create_table_row(row_data, is_header);
        table = table.push_row(table_row);
    }

    table
}

/// Create a table row from a vector of cell strings
///
/// # Parameters
/// * `cells` - Vector of strings, one per cell
/// * `is_header` - If true, text will be formatted in bold
///
/// # Returns
/// * `TableRow` - A formatted table row
fn create_table_row(cells: &[String], is_header: bool) -> TableRow<'static> {
    let mut row = TableRow::default();

    for cell_text in cells {
        let cell = create_table_cell(cell_text, is_header);
        row = row.push_cell(cell);
    }

    row
}

/// Create a table cell with text content
///
/// # Parameters
/// * `text` - The text content for the cell
/// * `bold` - If true, text will be formatted in bold
///
/// # Returns
/// * `TableCell` - A formatted table cell containing a paragraph with the text
fn create_table_cell(text: &str, bold: bool) -> TableCell<'static> {
    let mut prop = CharacterProperty::default();
    if bold {
        prop = prop.bold(true);
    }

    let run = Run::default()
        .property(prop)
        .push_text((text.to_string(), TextSpace::Preserve));

    let para = Paragraph::default().push(run);

    TableCell::from(para)
}

/// Convert Alignment to JustificationVal for paragraph formatting
fn alignment_to_justification(alignment: Alignment) -> JustificationVal {
    match alignment {
        Alignment::Left | Alignment::None => JustificationVal::Left,
        Alignment::Center => JustificationVal::Center,
        Alignment::Right => JustificationVal::Right,
    }
}

/// Create a DOCX table from inline markdown table data
///
/// Converts markdown table with formatted text runs into a DOCX table.
/// Headers are formatted in bold. Column alignments are preserved.
///
/// # Parameters
/// * `alignments` - Column alignment specifications
/// * `headers` - Header row cells, each containing formatted text runs
/// * `rows` - Data rows with cells containing formatted text runs
///
/// # Returns
/// * `Table` - A formatted DOCX table ready to be inserted into the document
fn create_inline_table(
    alignments: &[Alignment],
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
) -> Table<'static> {
    let mut table = Table::default();

    // Determine number of columns from headers
    let num_cols = headers.len();

    // Create table grid with equal column widths
    let mut grid = TableGrid::default();
    for _ in 0..num_cols {
        grid = grid.push_column(2000);
    }
    table.grids = grid;

    // Add header row (with bold formatting)
    let header_row = create_inline_table_row(headers, alignments, true);
    table = table.push_row(header_row);

    // Add data rows
    for row_data in rows {
        let data_row = create_inline_table_row(row_data, alignments, false);
        table = table.push_row(data_row);
    }

    table
}

/// Create a table row from cells containing formatted text runs
///
/// # Parameters
/// * `cells` - Vector of cells, each containing formatted text runs
/// * `alignments` - Column alignment specifications
/// * `is_header` - If true, text will be additionally formatted in bold
///
/// # Returns
/// * `TableRow` - A formatted table row
fn create_inline_table_row(
    cells: &[Vec<TextRun>],
    alignments: &[Alignment],
    is_header: bool,
) -> TableRow<'static> {
    let mut row = TableRow::default();

    for (col_idx, cell_runs) in cells.iter().enumerate() {
        let alignment = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
        let cell = create_inline_table_cell(cell_runs, alignment, is_header);
        row = row.push_cell(cell);
    }

    row
}

/// Create a table cell from formatted text runs
///
/// # Parameters
/// * `runs` - The formatted text runs for the cell content
/// * `alignment` - The horizontal alignment for the cell
/// * `make_bold` - If true, adds bold formatting to all runs
///
/// # Returns
/// * `TableCell` - A formatted table cell containing a paragraph with the text
fn create_inline_table_cell(
    runs: &[TextRun],
    alignment: Alignment,
    make_bold: bool,
) -> TableCell<'static> {
    let justification = alignment_to_justification(alignment);
    let mut para =
        Paragraph::default().property(ParagraphProperty::default().justification(justification));

    if runs.is_empty() {
        // Empty cells still need at least one run for valid DOCX
        let mut prop = CharacterProperty::default();
        if make_bold {
            prop = prop.bold(true);
        }
        let run = Run::default()
            .property(prop)
            .push_text((String::new(), TextSpace::Preserve));
        para = para.push(run);
    } else {
        for text_run in runs {
            let run = create_text_run(text_run, make_bold);
            para = para.push(run);
        }
    }

    TableCell::from(para)
}

/// Create a DOCX Run from a TextRun with formatting
fn create_text_run(text_run: &TextRun, make_bold: bool) -> Run<'static> {
    let text = text_run.text.clone();
    let mut prop = CharacterProperty::default();

    // Apply bold if this is a header row OR if the text run itself is bold
    if make_bold || text_run.bold {
        prop = prop.bold(true);
    }
    if text_run.italic {
        prop = prop.italics(true);
    }
    if text_run.strikethrough {
        prop = prop.strike(true);
    }
    if text_run.code {
        prop = prop.fonts(Fonts::default().ascii("Consolas").h_ansi("Consolas"));
    }

    Run::default()
        .property(prop)
        .push_text((text, TextSpace::Preserve))
}

/// Indentation in twips (twentieths of a point) per nesting level
/// 720 twips = 0.5 inch
const INDENT_TWIPS_PER_LEVEL: isize = 720;

/// Append a list (ordered or unordered) to the document
///
/// # Parameters
/// * `docx` - The document to append to
/// * `start` - Starting number for ordered lists (None for unordered)
/// * `items` - List items to append
/// * `indent_level` - Current indentation level for nested lists
/// * `image_lookup` - Image lookup table for any embedded images
fn append_list(
    docx: &mut Docx<'_>,
    start: &Option<u64>,
    items: &[ListItem],
    indent_level: usize,
    image_lookup: &HashMap<&PathBuf, ImageLookupInfo<'_>>,
) {
    let is_ordered = start.is_some();
    let start_num = start.unwrap_or(1);

    for (idx, item) in items.iter().enumerate() {
        let item_number = start_num + idx as u64;

        // Process each block in the list item
        for (block_idx, block) in item.content.iter().enumerate() {
            // Only the first block gets the bullet/number prefix
            let is_first_block = block_idx == 0;

            match block {
                MarkdownBlock::Paragraph(runs) => {
                    let para = create_list_item_paragraph(
                        runs,
                        is_ordered,
                        item_number,
                        indent_level,
                        is_first_block,
                    );
                    docx.document.push(para);
                }
                MarkdownBlock::List {
                    start: nested_start,
                    items: nested_items,
                } => {
                    // Recursively handle nested lists with increased indentation
                    append_list(
                        docx,
                        nested_start,
                        nested_items,
                        indent_level + 1,
                        image_lookup,
                    );
                }
                _ => {
                    // For other block types within list items, render with indentation
                    append_block_with_indent(docx, block, indent_level + 1, image_lookup);
                }
            }
        }
    }
}

/// Create a paragraph for a list item with bullet/number prefix
///
/// # Parameters
/// * `runs` - Text runs for the paragraph content
/// * `is_ordered` - Whether this is an ordered (numbered) list
/// * `item_number` - The number for ordered lists
/// * `indent_level` - Indentation level for nested lists
/// * `include_marker` - Whether to include the bullet/number marker
fn create_list_item_paragraph(
    runs: &[TextRun],
    is_ordered: bool,
    item_number: u64,
    indent_level: usize,
    include_marker: bool,
) -> Paragraph<'static> {
    let indent = Indent {
        left: Some((indent_level as isize + 1) * INDENT_TWIPS_PER_LEVEL),
        hanging: if include_marker {
            Some(INDENT_TWIPS_PER_LEVEL / 2)
        } else {
            None
        },
        ..Default::default()
    };

    let prop = ParagraphProperty::default().indent(indent);
    let mut para = Paragraph::default().property(prop);

    // Add bullet or number prefix
    if include_marker {
        let marker = if is_ordered {
            format!("{}.\t", item_number)
        } else {
            "•\t".to_string()
        };

        let marker_run = Run::default().push_text((marker, TextSpace::Preserve));
        para = para.push(marker_run);
    }

    // Add the content runs
    for text_run in runs {
        let run = create_run(text_run);
        para = para.push(run);
    }

    para
}

/// Append a block with indentation (for block quotes and nested content)
fn append_block_with_indent(
    docx: &mut Docx<'_>,
    block: &MarkdownBlock,
    indent_level: usize,
    image_lookup: &HashMap<&PathBuf, ImageLookupInfo<'_>>,
) {
    match block {
        MarkdownBlock::Paragraph(runs) => {
            let para = create_indented_paragraph(runs, indent_level);
            docx.document.push(para);
        }
        MarkdownBlock::List { start, items } => {
            append_list(docx, start, items, indent_level, image_lookup);
        }
        MarkdownBlock::BlockQuote(inner_blocks) => {
            for inner_block in inner_blocks {
                append_block_with_indent(docx, inner_block, indent_level + 1, image_lookup);
            }
        }
        _ => {
            // For other block types, just call the regular append_block
            // This handles images, tables, etc. within indented contexts
            append_block(docx, block, image_lookup);
        }
    }
}

/// Create an indented paragraph
fn create_indented_paragraph(runs: &[TextRun], indent_level: usize) -> Paragraph<'static> {
    let indent = Indent {
        left: Some((indent_level as isize) * INDENT_TWIPS_PER_LEVEL),
        ..Default::default()
    };

    let prop = ParagraphProperty::default().indent(indent);
    let mut para = Paragraph::default().property(prop);

    for text_run in runs {
        let run = create_run(text_run);
        para = para.push(run);
    }

    para
}

/// Create a paragraph for a code block with monospace formatting
fn create_code_block_paragraph(code: &str) -> Paragraph<'static> {
    let mut para = Paragraph::default();

    // Split code into lines and add each with monospace font
    for (idx, line) in code.lines().enumerate() {
        if idx > 0 {
            // Add a line break between lines
            // Note: In DOCX, we typically create separate paragraphs or use break elements
            // For simplicity, we'll use soft line breaks within a single paragraph
            let break_run = Run::default().push_break(docx_rust::document::BreakType::TextWrapping);
            para = para.push(break_run);
        }

        let prop = CharacterProperty::default()
            .fonts(Fonts::default().ascii("Consolas").h_ansi("Consolas"));

        let run = Run::default()
            .property(prop)
            .push_text((line.to_string(), TextSpace::Preserve));
        para = para.push(run);
    }

    para
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
