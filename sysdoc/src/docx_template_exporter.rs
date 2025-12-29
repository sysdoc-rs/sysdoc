//! DOCX export that preserves template styles and title page
//!
//! This module exports documents to .docx format by directly manipulating the XML
//! inside the ZIP archive, preserving the template's title page, styles, headers,
//! footers, and other formatting that libraries like docx-rust and docx-rs don't preserve.
//!
//! # Approach
//! A .docx file is a ZIP archive containing XML files. This exporter:
//! 1. Opens the template .docx as a ZIP archive
//! 2. Copies all template files to the output (preserving styles, theme, headers, etc.)
//! 3. Parses document.xml to find where to append content
//! 4. Generates content as raw OOXML
//! 5. Appends content to document.xml while preserving existing structure
//! 6. Updates relationships for embedded images
//!
//! This approach preserves:
//! - Title pages
//! - Custom styles (beyond just Heading1-9)
//! - Headers and footers
//! - Theme colors and fonts
//! - Document properties

use crate::source_model::{Alignment, ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::{DocumentMetadata, UnifiedDocument};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Export errors
#[derive(Debug)]
pub enum ExportError {
    /// I/O error during file operations
    Io(std::io::Error),
    /// ZIP archive error
    Zip(zip::result::ZipError),
    /// Document format error
    Format(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "IO error: {}", e),
            ExportError::Zip(e) => write!(f, "ZIP error: {}", e),
            ExportError::Format(msg) => write!(f, "Format error: {}", msg),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        ExportError::Io(e)
    }
}

impl From<zip::result::ZipError> for ExportError {
    fn from(e: zip::result::ZipError) -> Self {
        ExportError::Zip(e)
    }
}

/// EMUs (English Metric Units) per inch - Word uses this for measurements
const EMUS_PER_INCH: i64 = 914400;

/// Default DPI for images without embedded DPI information
const DEFAULT_IMAGE_DPI: f64 = 96.0;

/// Maximum image width in inches (to fit on a standard page with margins)
const MAX_IMAGE_WIDTH_INCHES: f64 = 6.5;

/// Pre-loaded image data for embedding
struct ImageData {
    bytes: Vec<u8>,
    extension: String,
    rel_id: String,
    width_emu: i64,
    height_emu: i64,
}

/// Export to Microsoft Word (.docx) preserving template styles
///
/// This exporter manipulates the DOCX ZIP archive directly to preserve
/// the template's title page, styles, and other formatting.
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `template_path` - Path to a .docx template file
/// * `output_path` - Path where the .docx file will be written
pub fn to_docx(
    doc: &UnifiedDocument,
    template_path: &Path,
    output_path: &Path,
) -> Result<(), ExportError> {
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    log::info!(
        "Template exporter: Reading template from {}",
        template_path.display()
    );

    // Collect images
    let images = collect_images(&doc.sections);
    log::info!("Collected {} images for embedding", images.len());

    // Generate content XML
    let content_xml = generate_content_xml(&doc.sections, &images);

    // Open template and create output
    let template_file = std::fs::File::open(template_path)?;
    let mut template_zip = ZipArchive::new(template_file)?;

    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output_file = std::fs::File::create(output_path)?;
    let mut output_zip = ZipWriter::new(output_file);

    // Track which files we've written (to handle document.xml and rels specially)
    let mut written_files: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Track docProps file contents for later updating
    let mut core_xml_contents: Option<Vec<u8>> = None;
    let mut custom_xml_contents: Option<Vec<u8>> = None;

    // Copy all files from template, modifying document.xml and relationships
    for i in 0..template_zip.len() {
        let mut file = template_zip.by_index(i)?;
        let name = file.name().to_string();

        // Skip directories
        if name.ends_with('/') {
            continue;
        }

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let modified_contents = if name == "word/document.xml" {
            // Inject our content into document.xml
            inject_content_into_document_xml(&contents, &content_xml)?
        } else if name == "word/_rels/document.xml.rels" {
            // Add image relationships
            add_image_relationships(&contents, &images)?
        } else if name == "[Content_Types].xml" {
            // Ensure image content types and docProps overrides are present
            let with_images = ensure_image_content_types(&contents, &images)?;
            ensure_docprops_content_types(&with_images)?
        } else if name == "_rels/.rels" {
            // Ensure docProps relationships are present
            ensure_docprops_relationships(&contents)?
        } else if name == "word/styles.xml" {
            // Ensure required styles (like Caption) are defined
            ensure_required_styles(&contents)?
        } else if name == "docProps/core.xml" {
            // Save for later updating, don't write yet
            core_xml_contents = Some(contents.clone());
            Vec::new() // Return empty to skip writing
        } else if name == "docProps/custom.xml" {
            // Save for later updating, don't write yet
            custom_xml_contents = Some(contents.clone());
            Vec::new() // Return empty to skip writing
        } else if name == "docProps/app.xml" {
            // Pass through unchanged
            contents
        } else {
            contents
        };

        // Only write if we have contents (skip empty ones like core.xml/custom.xml)
        if !modified_contents.is_empty() {
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            output_zip.start_file(&name, options)?;
            output_zip.write_all(&modified_contents)?;
            written_files.insert(name);
        }
    }

    // Write updated docProps files with document metadata
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let core_xml = update_or_create_core_properties(core_xml_contents.as_deref(), &doc.metadata)?;
    output_zip.start_file("docProps/core.xml", options)?;
    output_zip.write_all(&core_xml)?;
    written_files.insert("docProps/core.xml".to_string());

    let custom_xml =
        update_or_create_custom_properties(custom_xml_contents.as_deref(), &doc.metadata)?;
    output_zip.start_file("docProps/custom.xml", options)?;
    output_zip.write_all(&custom_xml)?;
    written_files.insert("docProps/custom.xml".to_string());

    // Add new image files to word/media/
    for (path, image_data) in &images {
        let media_path = format!(
            "word/media/image_{}.{}",
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown"),
            image_data.extension
        );

        if !written_files.contains(&media_path) {
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored); // Images don't compress well
            output_zip.start_file(&media_path, options)?;
            output_zip.write_all(&image_data.bytes)?;
        }
    }

    output_zip.finish()?;

    log::info!(
        "Successfully wrote DOCX with {} sections",
        doc.sections.len()
    );
    Ok(())
}

/// Try to load image data from a path
fn try_load_image(absolute_path: &Path, rel_id: usize) -> Option<ImageData> {
    let bytes = std::fs::read(absolute_path).ok()?;
    let extension = absolute_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let (width_emu, height_emu) = calculate_image_dimensions(&bytes);

    Some(ImageData {
        bytes,
        extension,
        rel_id: format!("rId{}", rel_id),
        width_emu,
        height_emu,
    })
}

/// Collect and load all images from document sections
fn collect_images(sections: &[MarkdownSection]) -> HashMap<PathBuf, ImageData> {
    let mut images = HashMap::new();
    let mut rel_id_counter = 100; // Start high to avoid conflicts

    let image_blocks = sections
        .iter()
        .flat_map(|s| &s.content)
        .filter_map(|block| {
            if let MarkdownBlock::Image {
                absolute_path,
                exists: true,
                ..
            } = block
            {
                Some(absolute_path)
            } else {
                None
            }
        });

    for absolute_path in image_blocks {
        if images.contains_key(absolute_path) {
            continue;
        }
        if let Some(image_data) = try_load_image(absolute_path, rel_id_counter) {
            images.insert(absolute_path.clone(), image_data);
            rel_id_counter += 1;
        }
    }

    images
}

/// Calculate image dimensions in EMUs, preserving aspect ratio
fn calculate_image_dimensions(bytes: &[u8]) -> (i64, i64) {
    match imagesize::blob_size(bytes) {
        Ok(size) if size.width > 0 && size.height > 0 => {
            let natural_width_inches = size.width as f64 / DEFAULT_IMAGE_DPI;
            let aspect_ratio = size.height as f64 / size.width as f64;

            let final_width_inches = natural_width_inches.min(MAX_IMAGE_WIDTH_INCHES);
            let final_height_inches = final_width_inches * aspect_ratio;

            let width_emu = (final_width_inches * EMUS_PER_INCH as f64) as i64;
            let height_emu = (final_height_inches * EMUS_PER_INCH as f64) as i64;
            (width_emu, height_emu)
        }
        _ => {
            // Fallback to 6x4 inches
            let width_emu = (6.0 * EMUS_PER_INCH as f64) as i64;
            let height_emu = (4.0 * EMUS_PER_INCH as f64) as i64;
            (width_emu, height_emu)
        }
    }
}

/// Generate OOXML content for all sections
fn generate_content_xml(
    sections: &[MarkdownSection],
    images: &HashMap<PathBuf, ImageData>,
) -> String {
    let mut xml = String::new();

    for section in sections {
        // Generate heading
        let heading_level = section.section_number.depth() + 1;
        let style_id = format!("Heading{}", heading_level.min(9));
        let heading_text = format!("{} {}", section.section_number, section.heading_text);

        xml.push_str(&format!(
            r#"<w:p><w:pPr><w:pStyle w:val="{}"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
            style_id,
            escape_xml(&heading_text)
        ));

        // Generate content blocks
        for block in &section.content {
            xml.push_str(&generate_block_xml(block, images));
        }
    }

    xml
}

/// Generate OOXML for a single block
fn generate_block_xml(block: &MarkdownBlock, images: &HashMap<PathBuf, ImageData>) -> String {
    match block {
        MarkdownBlock::Paragraph(runs) => generate_paragraph_xml(runs),
        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            title,
            exists: true,
            ..
        } => {
            if let Some(image_data) = images.get(absolute_path) {
                generate_image_xml(image_data, alt_text, title)
            } else {
                generate_paragraph_xml(&[TextRun::new(format!(
                    "[Image not found: {}]",
                    absolute_path.display()
                ))])
            }
        }
        MarkdownBlock::Image {
            absolute_path,
            exists: false,
            ..
        } => generate_paragraph_xml(&[TextRun::new(format!(
            "[Missing image: {}]",
            absolute_path.display()
        ))]),
        MarkdownBlock::CsvTable { data: Some(data), .. } if !data.is_empty() => {
            generate_table_xml(data)
        }
        MarkdownBlock::CsvTable { path, .. } => generate_paragraph_xml(&[TextRun::new(format!(
            "[CSV table: {}]",
            path.display()
        ))]),
        MarkdownBlock::InlineTable {
            alignments,
            headers,
            rows,
        } => generate_inline_table_xml(alignments, headers, rows),
        MarkdownBlock::List { start, items } => generate_list_xml(start, items, 0, images),
        MarkdownBlock::CodeBlock { code, .. } => generate_code_block_xml(code),
        MarkdownBlock::BlockQuote(blocks) => {
            let mut xml = String::new();
            for inner_block in blocks {
                xml.push_str(&generate_indented_block_xml(inner_block, 1, images));
            }
            xml
        }
        MarkdownBlock::Rule => {
            r#"<w:p><w:r><w:t>────────────────────────────────────────────────────</w:t></w:r></w:p>"#.to_string()
        }
        _ => generate_paragraph_xml(&[TextRun::new(format!(
            "[{:?} not implemented]",
            block_type_name(block)
        ))]),
    }
}

/// Generate OOXML for an indented block (used in block quotes)
fn generate_indented_block_xml(
    block: &MarkdownBlock,
    indent_level: usize,
    images: &HashMap<PathBuf, ImageData>,
) -> String {
    let indent_twips = indent_level * 720; // 720 twips = 0.5 inch

    match block {
        MarkdownBlock::Paragraph(runs) => {
            let mut xml = format!(r#"<w:p><w:pPr><w:ind w:left="{}"/></w:pPr>"#, indent_twips);
            for run in runs {
                xml.push_str(&generate_run_xml(run));
            }
            xml.push_str("</w:p>");
            xml
        }
        MarkdownBlock::BlockQuote(inner_blocks) => {
            let mut xml = String::new();
            for inner_block in inner_blocks {
                xml.push_str(&generate_indented_block_xml(
                    inner_block,
                    indent_level + 1,
                    images,
                ));
            }
            xml
        }
        _ => generate_block_xml(block, images),
    }
}

/// Generate OOXML for a paragraph
fn generate_paragraph_xml(runs: &[TextRun]) -> String {
    let mut xml = String::from("<w:p>");
    for run in runs {
        xml.push_str(&generate_run_xml(run));
    }
    xml.push_str("</w:p>");
    xml
}

/// Generate OOXML for a text run with formatting
fn generate_run_xml(run: &TextRun) -> String {
    let mut xml = String::from("<w:r>");

    // Build run properties if any formatting is applied
    if run.bold || run.italic || run.strikethrough || run.code {
        xml.push_str("<w:rPr>");
        if run.bold {
            xml.push_str("<w:b/>");
        }
        if run.italic {
            xml.push_str("<w:i/>");
        }
        if run.strikethrough {
            xml.push_str("<w:strike/>");
        }
        if run.code {
            xml.push_str(r#"<w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/>"#);
        }
        xml.push_str("</w:rPr>");
    }

    xml.push_str(&format!(
        r#"<w:t xml:space="preserve">{}</w:t>"#,
        escape_xml(&run.text)
    ));
    xml.push_str("</w:r>");
    xml
}

/// Generate OOXML for an inline image with caption
///
/// # Parameters
/// * `image_data` - The image data including dimensions and relationship ID
/// * `alt_text` - Alternative text for accessibility (used in image description)
/// * `title` - Title text used for the visible caption below the image
fn generate_image_xml(image_data: &ImageData, alt_text: &str, title: &str) -> String {
    // Use a static counter for unique IDs within a document export session
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let image_paragraph = format!(
        r#"<w:p>
  <w:pPr><w:jc w:val="center"/></w:pPr>
  <w:r>
    <w:drawing>
      <wp:inline distT="0" distB="0" distL="0" distR="0" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
        <wp:extent cx="{}" cy="{}"/>
        <wp:docPr id="{}" name="Picture {}" descr="{}"/>
        <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
              <pic:nvPicPr>
                <pic:cNvPr id="{}" name="Picture {}"/>
                <pic:cNvPicPr/>
              </pic:nvPicPr>
              <pic:blipFill>
                <a:blip r:embed="{}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
                <a:stretch><a:fillRect/></a:stretch>
              </pic:blipFill>
              <pic:spPr>
                <a:xfrm>
                  <a:ext cx="{}" cy="{}"/>
                </a:xfrm>
                <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
              </pic:spPr>
            </pic:pic>
          </a:graphicData>
        </a:graphic>
      </wp:inline>
    </w:drawing>
  </w:r>
</w:p>"#,
        image_data.width_emu,
        image_data.height_emu,
        id,
        id,
        escape_xml(alt_text), // Alt text for accessibility
        id,
        id,
        image_data.rel_id,
        image_data.width_emu,
        image_data.height_emu,
    );

    // Generate caption paragraph if title is not empty
    // Uses the Caption paragraph style which applies CaptionChar character formatting
    let caption_paragraph = if !title.is_empty() {
        format!(
            r#"<w:p>
  <w:pPr>
    <w:pStyle w:val="Caption"/>
    <w:jc w:val="center"/>
  </w:pPr>
  <w:r>
    <w:t xml:space="preserve">{}</w:t>
  </w:r>
</w:p>"#,
            escape_xml(title)
        )
    } else {
        String::new()
    };

    format!("{}{}", image_paragraph, caption_paragraph)
}

/// Generate OOXML for a table from CSV data
fn generate_table_xml(data: &[Vec<String>]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let num_cols = data.first().map(|r| r.len()).unwrap_or(0);

    let mut xml = String::from(
        r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="0" w:type="auto"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/>
    </w:tblBorders>
  </w:tblPr>
  <w:tblGrid>"#,
    );

    // Add grid columns (required for valid OOXML)
    for _ in 0..num_cols {
        xml.push_str("<w:gridCol/>");
    }
    xml.push_str("</w:tblGrid>");

    for (row_idx, row) in data.iter().enumerate() {
        let is_header = row_idx == 0;
        xml.push_str("<w:tr>");
        for cell in row {
            xml.push_str("<w:tc><w:p>");
            if is_header {
                xml.push_str(&format!(
                    r#"<w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
                    escape_xml(cell)
                ));
            } else {
                xml.push_str(&format!(
                    r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#,
                    escape_xml(cell)
                ));
            }
            xml.push_str("</w:p></w:tc>");
        }
        xml.push_str("</w:tr>");
    }

    xml.push_str("</w:tbl>");
    xml
}

/// Generate OOXML for an inline markdown table
fn generate_inline_table_xml(
    alignments: &[Alignment],
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
) -> String {
    let num_cols = headers.len();

    let mut xml = String::from(
        r#"<w:tbl>
  <w:tblPr>
    <w:tblW w:w="0" w:type="auto"/>
    <w:tblBorders>
      <w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      <w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/>
    </w:tblBorders>
  </w:tblPr>
  <w:tblGrid>"#,
    );

    // Add grid columns (required for valid OOXML)
    for _ in 0..num_cols {
        xml.push_str("<w:gridCol/>");
    }
    xml.push_str("</w:tblGrid>");

    // Header row
    xml.push_str("<w:tr>");
    for (idx, cell_runs) in headers.iter().enumerate() {
        let alignment = alignments.get(idx).copied().unwrap_or(Alignment::None);
        xml.push_str(&generate_table_cell_xml(cell_runs, alignment, true));
    }
    xml.push_str("</w:tr>");

    // Data rows
    for row in rows {
        xml.push_str("<w:tr>");
        for (idx, cell_runs) in row.iter().enumerate() {
            let alignment = alignments.get(idx).copied().unwrap_or(Alignment::None);
            xml.push_str(&generate_table_cell_xml(cell_runs, alignment, false));
        }
        xml.push_str("</w:tr>");
    }

    xml.push_str("</w:tbl>");
    xml
}

/// Generate OOXML for a table cell
fn generate_table_cell_xml(runs: &[TextRun], alignment: Alignment, is_header: bool) -> String {
    let align_val = match alignment {
        Alignment::Left | Alignment::None => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
    };

    let mut xml = format!(r#"<w:tc><w:p><w:pPr><w:jc w:val="{}"/></w:pPr>"#, align_val);

    for run in runs {
        let mut run_xml = String::from("<w:r><w:rPr>");
        if is_header || run.bold {
            run_xml.push_str("<w:b/>");
        }
        if run.italic {
            run_xml.push_str("<w:i/>");
        }
        if run.strikethrough {
            run_xml.push_str("<w:strike/>");
        }
        if run.code {
            run_xml.push_str(r#"<w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/>"#);
        }
        run_xml.push_str("</w:rPr>");
        run_xml.push_str(&format!(
            r#"<w:t xml:space="preserve">{}</w:t></w:r>"#,
            escape_xml(&run.text)
        ));
        xml.push_str(&run_xml);
    }

    xml.push_str("</w:p></w:tc>");
    xml
}

/// Generate the marker for a list item
fn generate_list_marker(is_ordered: bool, item_number: u64, is_first_block: bool) -> String {
    if !is_first_block {
        return String::new();
    }
    if is_ordered {
        format!("{}.\t", item_number)
    } else {
        "•\t".to_string()
    }
}

/// Generate OOXML for a list item paragraph
fn generate_list_paragraph_xml(runs: &[TextRun], marker: &str, indent_twips: usize) -> String {
    let mut xml = format!(
        r#"<w:p><w:pPr><w:ind w:left="{}" w:hanging="360"/></w:pPr>"#,
        indent_twips
    );

    if !marker.is_empty() {
        xml.push_str(&format!(
            r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#,
            escape_xml(marker)
        ));
    }

    for run in runs {
        xml.push_str(&generate_run_xml(run));
    }

    xml.push_str("</w:p>");
    xml
}

/// Generate OOXML for a list
fn generate_list_xml(
    start: &Option<u64>,
    items: &[ListItem],
    indent_level: usize,
    images: &HashMap<PathBuf, ImageData>,
) -> String {
    let mut xml = String::new();
    let is_ordered = start.is_some();
    let start_num = start.unwrap_or(1);
    let indent_twips = (indent_level + 1) * 720;

    for (idx, item) in items.iter().enumerate() {
        let item_number = start_num + idx as u64;
        xml.push_str(&generate_list_item_xml(
            item,
            is_ordered,
            item_number,
            indent_twips,
            indent_level,
            images,
        ));
    }

    xml
}

/// Generate OOXML for a single list item
fn generate_list_item_xml(
    item: &ListItem,
    is_ordered: bool,
    item_number: u64,
    indent_twips: usize,
    indent_level: usize,
    images: &HashMap<PathBuf, ImageData>,
) -> String {
    let mut xml = String::new();

    for (block_idx, block) in item.content.iter().enumerate() {
        let is_first_block = block_idx == 0;

        match block {
            MarkdownBlock::Paragraph(runs) => {
                let marker = generate_list_marker(is_ordered, item_number, is_first_block);
                xml.push_str(&generate_list_paragraph_xml(runs, &marker, indent_twips));
            }
            MarkdownBlock::List {
                start: nested_start,
                items: nested_items,
            } => {
                xml.push_str(&generate_list_xml(
                    nested_start,
                    nested_items,
                    indent_level + 1,
                    images,
                ));
            }
            _ => {
                xml.push_str(&generate_indented_block_xml(
                    block,
                    indent_level + 1,
                    images,
                ));
            }
        }
    }

    xml
}

/// Generate OOXML for a code block
fn generate_code_block_xml(code: &str) -> String {
    let mut xml = String::from("<w:p>");

    for (idx, line) in code.lines().enumerate() {
        if idx > 0 {
            xml.push_str("<w:r><w:br/></w:r>");
        }
        xml.push_str(&format!(
            r#"<w:r><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
            escape_xml(line)
        ));
    }

    xml.push_str("</w:p>");
    xml
}

/// Get block type name for error messages
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
    }
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Format ISO 8601 datetime for use in docProps with W3CDTF type
///
/// Simply validates the string isn't empty. Git already provides ISO 8601 format.
fn format_iso8601_for_docprops(iso_str: &str) -> Option<String> {
    if iso_str.trim().is_empty() {
        None
    } else {
        Some(iso_str.trim().to_string())
    }
}

/// Extract text content between XML tags
///
/// Helper function to extract preserved values from existing XML
fn extract_xml_tag_content(xml: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag_name);
    let close_tag = format!("</{}>", tag_name);

    if let Some(start) = xml.find(&open_tag) {
        let content_start = start + open_tag.len();
        if let Some(end) = xml[content_start..].find(&close_tag) {
            return Some(xml[content_start..content_start + end].to_string());
        }
    }
    None
}

/// Update or create docProps/core.xml with document metadata
///
/// This function either creates a new core.xml from scratch or updates an existing one,
/// preserving fields that we don't explicitly set.
fn update_or_create_core_properties(
    existing_xml: Option<&[u8]>,
    metadata: &DocumentMetadata,
) -> Result<Vec<u8>, ExportError> {
    // Extract preserved fields from existing XML
    let (last_modified_by, revision) = if let Some(xml_bytes) = existing_xml {
        let xml_str = String::from_utf8_lossy(xml_bytes);
        (
            extract_xml_tag_content(&xml_str, "cp:lastModifiedBy").unwrap_or_default(),
            extract_xml_tag_content(&xml_str, "cp:revision").unwrap_or_else(|| "1".to_string()),
        )
    } else {
        (String::new(), "1".to_string())
    };

    // Prepare values
    let title = escape_xml(&metadata.title);
    let creator = escape_xml(&metadata.owner.name);
    let subject = if let Some(subtitle) = &metadata.subtitle {
        escape_xml(subtitle)
    } else {
        String::new()
    };
    let description = if let Some(desc) = &metadata.description {
        escape_xml(desc)
    } else {
        String::new()
    };

    // Build keywords from system_id, document_id, document_type, and document_standard
    let mut keywords_parts = Vec::new();
    if let Some(sys_id) = &metadata.system_id {
        keywords_parts.push(sys_id.as_str());
    }
    keywords_parts.push(&metadata.document_id);
    keywords_parts.push(&metadata.doc_type);
    keywords_parts.push(&metadata.standard);
    let keywords = escape_xml(&keywords_parts.join(", "));

    // Build XML
    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(r#"<cp:coreProperties "#);
    xml.push_str(
        r#"xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" "#,
    );
    xml.push_str(r#"xmlns:dc="http://purl.org/dc/elements/1.1/" "#);
    xml.push_str(r#"xmlns:dcterms="http://purl.org/dc/terms/" "#);
    xml.push_str(r#"xmlns:dcmitype="http://purl.org/dc/dcmitype/" "#);
    xml.push_str(r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#);
    xml.push_str(&format!("<dc:title>{}</dc:title>", title));
    xml.push_str(&format!("<dc:subject>{}</dc:subject>", subject));
    xml.push_str(&format!("<dc:creator>{}</dc:creator>", creator));
    xml.push_str(&format!("<cp:keywords>{}</cp:keywords>", keywords));
    xml.push_str(&format!(
        "<dc:description>{}</dc:description>",
        escape_xml(&description)
    ));
    xml.push_str(&format!(
        "<cp:lastModifiedBy>{}</cp:lastModifiedBy>",
        escape_xml(&last_modified_by)
    ));
    xml.push_str(&format!(
        "<cp:revision>{}</cp:revision>",
        escape_xml(&revision)
    ));

    // Add created date if available
    if let Some(created) = &metadata.created {
        if let Some(formatted_date) = format_iso8601_for_docprops(created) {
            xml.push_str(&format!(
                r#"<dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>"#,
                escape_xml(&formatted_date)
            ));
        }
    }

    // Add modified date if available
    if let Some(modified) = &metadata.modified {
        if let Some(formatted_date) = format_iso8601_for_docprops(modified) {
            xml.push_str(&format!(
                r#"<dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>"#,
                escape_xml(&formatted_date)
            ));
        }
    }

    xml.push_str("</cp:coreProperties>");

    Ok(xml.into_bytes())
}

/// Extract name and value from a custom property XML fragment
///
/// Returns None if the property cannot be parsed or should not be preserved
fn extract_custom_property(
    prop_xml: &str,
    our_property_names: &std::collections::HashSet<&str>,
) -> Option<(String, String)> {
    // Extract name attribute
    let name_start = prop_xml.find(r#"name=""#)?;
    let name_content_start = name_start + r#"name=""#.len();
    let name_end = prop_xml[name_content_start..].find('"')?;
    let name = &prop_xml[name_content_start..name_content_start + name_end];

    // Skip if this is a property we're overriding
    if our_property_names.contains(name) {
        return None;
    }

    // Extract value (between > and </ of vt:lpwstr)
    let value_start = prop_xml.find("<vt:lpwstr>")?;
    let value_content_start = value_start + "<vt:lpwstr>".len();
    let value_end = prop_xml[value_content_start..].find("</vt:lpwstr>")?;
    let value = &prop_xml[value_content_start..value_content_start + value_end];

    Some((name.to_string(), value.to_string()))
}

/// Update or create docProps/custom.xml with document metadata
///
/// This function either creates new custom.xml from scratch or updates an existing one,
/// preserving properties that we don't explicitly override.
fn update_or_create_custom_properties(
    existing_xml: Option<&[u8]>,
    metadata: &DocumentMetadata,
) -> Result<Vec<u8>, ExportError> {
    // Standard GUID for custom properties
    const FMTID: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";

    // Define our properties (name, value)
    let mut our_properties: Vec<(&str, String)> = vec![
        ("Document number", metadata.document_id.clone()),
        ("Document Type", metadata.doc_type.clone()),
        ("Standard", metadata.standard.clone()),
        ("Owner", metadata.owner.name.clone()),
        ("Owner Email", metadata.owner.email.clone()),
        ("Approver", metadata.approver.name.clone()),
        ("Approver Email", metadata.approver.email.clone()),
    ];

    // Add Version if present
    if let Some(version) = &metadata.version {
        our_properties.push(("Version", version.to_string()));
    }

    // Build list of property names we want to override (including Version even if None)
    let mut our_property_names: std::collections::HashSet<&str> =
        our_properties.iter().map(|(name, _)| *name).collect();
    our_property_names.insert("Version"); // Always override Version from template

    // Parse existing properties to preserve non-overridden ones
    let preserved_properties: Vec<(String, String)> = if let Some(xml_bytes) = existing_xml {
        let xml_str = String::from_utf8_lossy(xml_bytes);
        let mut preserved = Vec::new();
        let mut search_pos = 0;

        while let Some(prop_start) = xml_str[search_pos..].find("<property ") {
            let abs_start = search_pos + prop_start;
            let Some(prop_end) = xml_str[abs_start..].find("</property>") else {
                break;
            };

            let prop_xml = &xml_str[abs_start..abs_start + prop_end + "</property>".len()];

            // Extract and preserve if not in our override list
            if let Some((name, value)) = extract_custom_property(prop_xml, &our_property_names) {
                preserved.push((name, value));
            }

            search_pos = abs_start + prop_end + "</property>".len();
        }

        preserved
    } else {
        Vec::new()
    };

    // Build XML
    let mut xml = String::with_capacity(2048);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(r#"<Properties "#);
    xml.push_str(
        r#"xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" "#,
    );
    xml.push_str(
        r#"xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">"#,
    );

    // Add our properties with sequential PIDs
    let mut pid = 2;
    for (name, value) in &our_properties {
        xml.push_str(&format!(
            r#"<property fmtid="{}" pid="{}" name="{}">"#,
            FMTID,
            pid,
            escape_xml(name)
        ));
        xml.push_str(&format!("<vt:lpwstr>{}</vt:lpwstr>", escape_xml(value)));
        xml.push_str("</property>");
        pid += 1;
    }

    // Add preserved properties
    for (name, value) in &preserved_properties {
        xml.push_str(&format!(
            r#"<property fmtid="{}" pid="{}" name="{}">"#,
            FMTID,
            pid,
            escape_xml(name)
        ));
        xml.push_str(&format!("<vt:lpwstr>{}</vt:lpwstr>", escape_xml(value)));
        xml.push_str("</property>");
        pid += 1;
    }

    xml.push_str("</Properties>");

    Ok(xml.into_bytes())
}

/// Ensure _rels/.rels contains relationships for docProps files
///
/// Adds relationships for core.xml and custom.xml if they don't already exist
fn ensure_docprops_relationships(rels_xml: &[u8]) -> Result<Vec<u8>, ExportError> {
    let xml_str = String::from_utf8_lossy(rels_xml);

    // Check if relationships already exist
    let has_core = xml_str.contains("docProps/core.xml");
    let has_custom = xml_str.contains("docProps/custom.xml");

    if has_core && has_custom {
        // Both already exist, return unchanged
        return Ok(rels_xml.to_vec());
    }

    // Find the closing </Relationships> tag
    let rels_close_pos = xml_str.rfind("</Relationships>").ok_or_else(|| {
        ExportError::Format("Could not find </Relationships> in _rels/.rels".to_string())
    })?;

    // Find next available rId number
    let mut max_rid = 0;
    let mut search_pos = 0;
    while let Some(rid_start) = xml_str[search_pos..].find(r#" Id="rId"#) {
        let abs_start = search_pos + rid_start + r#" Id="rId"#.len();
        // Extract digits after rId
        let mut digits = String::new();
        for ch in xml_str[abs_start..].chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }
        if let Ok(num) = digits.parse::<i32>() {
            max_rid = max_rid.max(num);
        }
        search_pos = abs_start;
    }

    let mut result = String::with_capacity(xml_str.len() + 500);
    result.push_str(&xml_str[..rels_close_pos]);

    // Add core.xml relationship if missing
    if !has_core {
        max_rid += 1;
        result.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>"#,
            max_rid
        ));
    }

    // Add custom.xml relationship if missing
    if !has_custom {
        max_rid += 1;
        result.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/>"#,
            max_rid
        ));
    }

    result.push_str(&xml_str[rels_close_pos..]);

    Ok(result.into_bytes())
}

/// Ensure \[Content_Types\].xml contains Override entries for docProps files
///
/// Adds Override elements for core.xml and custom.xml if they don't already exist
fn ensure_docprops_content_types(content_types_xml: &[u8]) -> Result<Vec<u8>, ExportError> {
    let xml_str = String::from_utf8_lossy(content_types_xml);

    // Check if overrides already exist
    let has_core = xml_str.contains(r#"PartName="/docProps/core.xml""#);
    let has_custom = xml_str.contains(r#"PartName="/docProps/custom.xml""#);

    if has_core && has_custom {
        // Both already exist, return unchanged
        return Ok(content_types_xml.to_vec());
    }

    // Find the closing </Types> tag
    let types_close_pos = xml_str.rfind("</Types>").ok_or_else(|| {
        ExportError::Format("Could not find </Types> in [Content_Types].xml".to_string())
    })?;

    let mut result = String::with_capacity(xml_str.len() + 400);
    result.push_str(&xml_str[..types_close_pos]);

    // Add core.xml override if missing
    if !has_core {
        result.push_str(
            r#"<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>"#
        );
    }

    // Add custom.xml override if missing
    if !has_custom {
        result.push_str(
            r#"<Override PartName="/docProps/custom.xml" ContentType="application/vnd.openxmlformats-officedocument.custom-properties+xml"/>"#
        );
    }

    result.push_str(&xml_str[types_close_pos..]);

    Ok(result.into_bytes())
}

/// Inject content XML into document.xml, preserving template structure
fn inject_content_into_document_xml(
    document_xml: &[u8],
    content_xml: &str,
) -> Result<Vec<u8>, ExportError> {
    let xml_str = String::from_utf8_lossy(document_xml);

    // Find the closing </w:body> tag and insert content before it
    // This preserves any existing template content (like title page) and appends our content
    if let Some(body_close_pos) = xml_str.rfind("</w:body>") {
        let mut result = String::with_capacity(xml_str.len() + content_xml.len());
        result.push_str(&xml_str[..body_close_pos]);
        result.push_str(content_xml);
        result.push_str(&xml_str[body_close_pos..]);
        Ok(result.into_bytes())
    } else {
        Err(ExportError::Format(
            "Could not find </w:body> in document.xml".to_string(),
        ))
    }
}

/// Add image relationships to document.xml.rels
fn add_image_relationships(
    rels_xml: &[u8],
    images: &HashMap<PathBuf, ImageData>,
) -> Result<Vec<u8>, ExportError> {
    if images.is_empty() {
        return Ok(rels_xml.to_vec());
    }

    let xml_str = String::from_utf8_lossy(rels_xml);

    // Find the closing </Relationships> tag
    if let Some(rels_close_pos) = xml_str.rfind("</Relationships>") {
        let mut result = String::with_capacity(xml_str.len() + images.len() * 200);
        result.push_str(&xml_str[..rels_close_pos]);

        // Add relationship entries for each image
        for (path, image_data) in images {
            let media_path = format!(
                "media/image_{}.{}",
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown"),
                image_data.extension
            );

            result.push_str(&format!(
                r#"<Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{}"/>"#,
                image_data.rel_id, media_path
            ));
        }

        result.push_str("</Relationships>");
        Ok(result.into_bytes())
    } else {
        Err(ExportError::Format(
            "Could not find </Relationships> in document.xml.rels".to_string(),
        ))
    }
}

/// Ensure required styles are present in styles.xml
fn ensure_required_styles(styles_xml: &[u8]) -> Result<Vec<u8>, ExportError> {
    let xml_str = String::from_utf8_lossy(styles_xml);

    // Check if Caption style already exists
    if xml_str.contains(r#"w:styleId="Caption""#) {
        return Ok(styles_xml.to_vec());
    }

    // Caption style definition - italic, centered, 10pt
    // Note: We don't use basedOn/next because the template may not have Normal defined
    let caption_style = r#"<w:style w:type="paragraph" w:styleId="Caption">
  <w:name w:val="Caption"/>
  <w:qFormat/>
  <w:pPr>
    <w:spacing w:before="0" w:after="200"/>
    <w:jc w:val="center"/>
  </w:pPr>
  <w:rPr>
    <w:i/>
    <w:iCs/>
    <w:sz w:val="20"/>
    <w:szCs w:val="20"/>
  </w:rPr>
</w:style>"#;

    // Find the closing </w:styles> tag and insert before it
    if let Some(styles_close_pos) = xml_str.rfind("</w:styles>") {
        let mut result = String::with_capacity(xml_str.len() + caption_style.len());
        result.push_str(&xml_str[..styles_close_pos]);
        result.push_str(caption_style);
        result.push_str("</w:styles>");
        Ok(result.into_bytes())
    } else {
        // If no closing tag found, return as-is
        Ok(styles_xml.to_vec())
    }
}

/// Ensure image content types are present in \[Content_Types\].xml
fn ensure_image_content_types(
    content_types_xml: &[u8],
    images: &HashMap<PathBuf, ImageData>,
) -> Result<Vec<u8>, ExportError> {
    if images.is_empty() {
        return Ok(content_types_xml.to_vec());
    }

    let xml_str = String::from_utf8_lossy(content_types_xml);

    // Collect unique extensions
    let extensions: std::collections::HashSet<&str> =
        images.values().map(|d| d.extension.as_str()).collect();

    // Find the closing </Types> tag
    if let Some(types_close_pos) = xml_str.rfind("</Types>") {
        let mut result = String::with_capacity(xml_str.len() + extensions.len() * 100);
        result.push_str(&xml_str[..types_close_pos]);

        // Add content type entries for each unique extension if not already present
        for ext in extensions {
            let content_type = match ext {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };

            // Only add if not already present
            let default_entry = format!(r#"Extension="{}""#, ext);
            if !xml_str.contains(&default_entry) {
                result.push_str(&format!(
                    r#"<Default Extension="{}" ContentType="{}"/>"#,
                    ext, content_type
                ));
            }
        }

        result.push_str("</Types>");
        Ok(result.into_bytes())
    } else {
        Err(ExportError::Format(
            "Could not find </Types> in [Content_Types].xml".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_required_styles_adds_caption() {
        // Minimal styles.xml without Caption
        let input = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"></w:styles>"#;

        let result = ensure_required_styles(input).unwrap();
        let output = String::from_utf8(result).unwrap();

        assert!(output.contains(r#"w:styleId="Caption""#));
        assert!(output.contains("<w:i/>"));
    }

    #[test]
    fn test_ensure_required_styles_preserves_existing_caption() {
        // styles.xml with existing Caption style
        let input = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:styleId="Caption"><w:name w:val="Caption"/></w:style>
</w:styles>"#;

        let result = ensure_required_styles(input).unwrap();

        // Should return unchanged
        assert_eq!(result, input.to_vec());
    }
}
