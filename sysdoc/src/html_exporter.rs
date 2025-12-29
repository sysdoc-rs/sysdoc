//! HTML exporter for aggregated documents
//!
//! This module exports a UnifiedDocument to a single HTML file with:
//! - Numbered headings (using section numbers like 1.2.3)
//! - Images embedded as data URLs (base64 encoded)
//! - Modern CSS styling with sans-serif fonts

use crate::source_model::{Alignment, ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during HTML export
#[derive(Error, Debug)]
pub enum HtmlExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image read error for {path}: {source}", path = .path.display())]
    ImageReadError {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Export a unified document to HTML format
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `output_path` - Path where the HTML file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to HTML
/// * `Err(HtmlExportError)` - Error during export
pub fn to_html(doc: &UnifiedDocument, output_path: &Path) -> Result<(), HtmlExportError> {
    let mut output = String::new();

    // Write HTML header with CSS
    write_html_header(&mut output, &doc.metadata.title);

    // Start body
    output.push_str("<body>\n");
    output.push_str("<div class=\"container\">\n");

    // Write document title as H1 if available
    if !doc.metadata.title.is_empty() {
        output.push_str(&format!(
            "<h1 class=\"document-title\">{}</h1>\n",
            escape_html(&doc.metadata.title)
        ));
    }

    // Write document metadata
    write_metadata(&mut output, doc);

    // Write each section
    for section in &doc.sections {
        write_section(&mut output, section)?;
    }

    // Close container and body
    output.push_str("</div>\n");
    output.push_str("</body>\n");
    output.push_str("</html>\n");

    // Write to file - create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(output_path)?;
    file.write_all(output.as_bytes())?;

    Ok(())
}

/// Write HTML header with CSS styling
fn write_html_header(output: &mut String, title: &str) {
    output.push_str("<!DOCTYPE html>\n");
    output.push_str("<html lang=\"en\">\n");
    output.push_str("<head>\n");
    output.push_str("<meta charset=\"UTF-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    output.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    output.push_str("<style>\n");
    output.push_str(CSS_STYLES);
    output.push_str("</style>\n");
    output.push_str("</head>\n");
}

/// Write document metadata section
fn write_metadata(output: &mut String, doc: &UnifiedDocument) {
    output.push_str("<div class=\"metadata\">\n");

    if let Some(ref subtitle) = doc.metadata.subtitle {
        output.push_str(&format!(
            "<p class=\"subtitle\">{}</p>\n",
            escape_html(subtitle)
        ));
    }

    if let Some(ref description) = doc.metadata.description {
        output.push_str(&format!(
            "<p class=\"description\">{}</p>\n",
            escape_html(description)
        ));
    }

    output.push_str("<table class=\"metadata-table\">\n");

    output.push_str(&format!(
        "<tr><td class=\"label\">Document ID:</td><td>{}</td></tr>\n",
        escape_html(&doc.metadata.document_id)
    ));

    output.push_str(&format!(
        "<tr><td class=\"label\">Type:</td><td>{}</td></tr>\n",
        escape_html(&doc.metadata.doc_type)
    ));

    output.push_str(&format!(
        "<tr><td class=\"label\">Standard:</td><td>{}</td></tr>\n",
        escape_html(&doc.metadata.standard)
    ));

    if let Some(ref system_id) = doc.metadata.system_id {
        output.push_str(&format!(
            "<tr><td class=\"label\">System ID:</td><td>{}</td></tr>\n",
            escape_html(system_id)
        ));
    }

    output.push_str(&format!(
        "<tr><td class=\"label\">Owner:</td><td>{} &lt;{}&gt;</td></tr>\n",
        escape_html(&doc.metadata.owner.name),
        escape_html(&doc.metadata.owner.email)
    ));

    output.push_str(&format!(
        "<tr><td class=\"label\">Approver:</td><td>{} &lt;{}&gt;</td></tr>\n",
        escape_html(&doc.metadata.approver.name),
        escape_html(&doc.metadata.approver.email)
    ));

    if let Some(ref version) = doc.metadata.version {
        output.push_str(&format!(
            "<tr><td class=\"label\">Version:</td><td>{}</td></tr>\n",
            escape_html(version)
        ));
    }

    if let Some(ref created) = doc.metadata.created {
        output.push_str(&format!(
            "<tr><td class=\"label\">Created:</td><td>{}</td></tr>\n",
            escape_html(created)
        ));
    }

    if let Some(ref modified) = doc.metadata.modified {
        output.push_str(&format!(
            "<tr><td class=\"label\">Modified:</td><td>{}</td></tr>\n",
            escape_html(modified)
        ));
    }

    output.push_str("</table>\n");
    output.push_str("</div>\n");
}

/// Write a single section to the output
fn write_section(output: &mut String, section: &MarkdownSection) -> Result<(), HtmlExportError> {
    // Determine heading level (h1-h6)
    let level = section.heading_level.min(6);

    // Write heading with section number
    output.push_str(&format!(
        "<h{} class=\"section-heading\"><span class=\"section-number\">{}</span> {}</h{}>\n",
        level,
        escape_html(&section.section_number.to_string()),
        escape_html(&section.heading_text),
        level
    ));

    // Write content blocks
    for block in &section.content {
        write_block(output, block, 0)?;
    }

    Ok(())
}

/// Write a single block to the output
fn write_block(
    output: &mut String,
    block: &MarkdownBlock,
    indent_level: usize,
) -> Result<(), HtmlExportError> {
    match block {
        MarkdownBlock::Heading { level, runs } => {
            let h_level = (*level).min(6);
            output.push_str(&format!(
                "<h{} class=\"content-heading\">{}</h{}>\n",
                h_level,
                runs_to_html(runs),
                h_level
            ));
        }

        MarkdownBlock::Paragraph(runs) => {
            output.push_str(&format!("<p>{}</p>\n", runs_to_html(runs)));
        }

        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            title,
            format,
            exists,
            ..
        } => {
            write_image(output, absolute_path, alt_text, title, format, *exists);
        }

        MarkdownBlock::CodeBlock {
            language,
            code,
            fenced: _,
        } => {
            if let Some(lang) = language {
                output.push_str(&format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>\n",
                    escape_html(lang),
                    escape_html(code)
                ));
            } else {
                output.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(code)));
            }
        }

        MarkdownBlock::BlockQuote(blocks) => {
            output.push_str("<blockquote>\n");
            for inner_block in blocks {
                write_block(output, inner_block, indent_level + 1)?;
            }
            output.push_str("</blockquote>\n");
        }

        MarkdownBlock::List { start, items } => {
            write_list(output, start, items, indent_level)?;
        }

        MarkdownBlock::InlineTable {
            alignments,
            headers,
            rows,
        } => {
            write_inline_table(output, alignments, headers, rows);
        }

        MarkdownBlock::CsvTable { data, .. } => {
            if let Some(table_data) = data {
                write_csv_table(output, table_data);
            }
        }

        MarkdownBlock::Rule => {
            output.push_str("<hr>\n");
        }

        MarkdownBlock::Html(html) => {
            output.push_str(html);
            output.push('\n');
        }
    }

    Ok(())
}

/// Write an image block to HTML output
fn write_image(
    output: &mut String,
    absolute_path: &Path,
    alt_text: &str,
    title: &str,
    format: &crate::source_model::ImageFormat,
    exists: bool,
) {
    if !exists {
        output.push_str(&format!(
            "<p class=\"image-error\">Image not found: {}</p>\n",
            escape_html(&absolute_path.display().to_string())
        ));
        return;
    }

    let data = match fs::read(absolute_path) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Failed to read image {}: {}", absolute_path.display(), e);
            output.push_str(&format!(
                "<p class=\"image-error\">Failed to read image: {}</p>\n",
                escape_html(&absolute_path.display().to_string())
            ));
            return;
        }
    };

    let mime_type = match format {
        crate::source_model::ImageFormat::Png => "image/png",
        crate::source_model::ImageFormat::Jpeg => "image/jpeg",
        crate::source_model::ImageFormat::Svg | crate::source_model::ImageFormat::DrawIoSvg => {
            "image/svg+xml"
        }
        crate::source_model::ImageFormat::Other => "application/octet-stream",
    };

    let base64_data = STANDARD.encode(&data);
    let data_url = format!("data:{};base64,{}", mime_type, base64_data);

    if title.is_empty() {
        output.push_str(&format!(
            "<figure><img src=\"{}\" alt=\"{}\"></figure>\n",
            data_url,
            escape_html(alt_text)
        ));
    } else {
        output.push_str(&format!(
            "<figure><img src=\"{}\" alt=\"{}\" title=\"{}\"><figcaption>{}</figcaption></figure>\n",
            data_url,
            escape_html(alt_text),
            escape_html(title),
            escape_html(title)
        ));
    }
}

/// Convert text runs to HTML string with formatting
fn runs_to_html(runs: &[TextRun]) -> String {
    let mut result = String::new();

    for run in runs {
        let mut text = escape_html(&run.text);

        // Apply formatting
        if run.code {
            text = format!("<code>{}</code>", text);
        }
        if run.bold {
            text = format!("<strong>{}</strong>", text);
        }
        if run.italic {
            text = format!("<em>{}</em>", text);
        }
        if run.strikethrough {
            text = format!("<del>{}</del>", text);
        }
        if run.superscript {
            text = format!("<sup>{}</sup>", text);
        }
        if run.subscript {
            text = format!("<sub>{}</sub>", text);
        }

        // Apply link if present
        if let Some(ref url) = run.link_url {
            let escaped_url = escape_html(url);
            if let Some(ref link_title) = run.link_title {
                text = format!(
                    "<a href=\"{}\" title=\"{}\">{}</a>",
                    escaped_url,
                    escape_html(link_title),
                    text
                );
            } else {
                text = format!("<a href=\"{}\">{}</a>", escaped_url, text);
            }
        }

        result.push_str(&text);
    }

    result
}

/// Write a list to output
fn write_list(
    output: &mut String,
    start: &Option<u64>,
    items: &[ListItem],
    indent_level: usize,
) -> Result<(), HtmlExportError> {
    if start.is_some() {
        // Ordered list
        if let Some(start_num) = start {
            output.push_str(&format!("<ol start=\"{}\">\n", start_num));
        } else {
            output.push_str("<ol>\n");
        }
    } else {
        // Check if this is a task list
        let is_task_list = items.iter().any(|item| item.task_list.is_some());
        if is_task_list {
            output.push_str("<ul class=\"task-list\">\n");
        } else {
            output.push_str("<ul>\n");
        }
    }

    for item in items {
        write_list_item(output, item, indent_level)?;
    }

    if start.is_some() {
        output.push_str("</ol>\n");
    } else {
        output.push_str("</ul>\n");
    }

    Ok(())
}

/// Write a single list item to output
fn write_list_item(
    output: &mut String,
    item: &ListItem,
    indent_level: usize,
) -> Result<(), HtmlExportError> {
    // Handle task list items
    if let Some(checked) = item.task_list {
        let checkbox = if checked {
            "<input type=\"checkbox\" checked disabled>"
        } else {
            "<input type=\"checkbox\" disabled>"
        };
        output.push_str(&format!("<li class=\"task-list-item\">{} ", checkbox));
    } else {
        output.push_str("<li>");
    }

    // Write first block inline with <li>
    let mut blocks = item.content.iter();
    if let Some(first_block) = blocks.next() {
        write_first_list_block(output, first_block)?;
    }

    // Write remaining blocks as nested content
    for block in blocks {
        write_block(output, block, indent_level + 1)?;
    }

    output.push_str("</li>\n");

    Ok(())
}

/// Write the first block of a list item (inline with `<li>`)
fn write_first_list_block(
    output: &mut String,
    block: &MarkdownBlock,
) -> Result<(), HtmlExportError> {
    if let MarkdownBlock::Paragraph(runs) = block {
        output.push_str(&runs_to_html(runs));
    } else {
        // For non-paragraph first blocks, write them normally
        write_block(output, block, 0)?;
    }
    Ok(())
}

/// Write an inline markdown table as HTML
fn write_inline_table(
    output: &mut String,
    alignments: &[Alignment],
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
) {
    output.push_str("<table>\n<thead>\n<tr>\n");

    // Write header row
    for (i, header) in headers.iter().enumerate() {
        let align = alignments.get(i).copied().unwrap_or(Alignment::None);
        let align_attr = get_align_attr(align);
        output.push_str(&format!(
            "<th{}>{}</th>\n",
            align_attr,
            runs_to_html(header)
        ));
    }

    output.push_str("</tr>\n</thead>\n<tbody>\n");

    // Write data rows
    for row in rows {
        output.push_str("<tr>\n");
        for (i, cell) in row.iter().enumerate() {
            let align = alignments.get(i).copied().unwrap_or(Alignment::None);
            let align_attr = get_align_attr(align);
            output.push_str(&format!("<td{}>{}</td>\n", align_attr, runs_to_html(cell)));
        }
        output.push_str("</tr>\n");
    }

    output.push_str("</tbody>\n</table>\n");
}

/// Write a CSV table as HTML
fn write_csv_table(output: &mut String, data: &[Vec<String>]) {
    if data.is_empty() {
        return;
    }

    output.push_str("<table>\n<thead>\n<tr>\n");

    // First row is headers
    let headers = &data[0];
    for header in headers {
        output.push_str(&format!("<th>{}</th>\n", escape_html(header)));
    }

    output.push_str("</tr>\n</thead>\n<tbody>\n");

    // Write data rows
    for row in data.iter().skip(1) {
        output.push_str("<tr>\n");
        for cell in row {
            output.push_str(&format!("<td>{}</td>\n", escape_html(cell)));
        }
        output.push_str("</tr>\n");
    }

    output.push_str("</tbody>\n</table>\n");
}

/// Get HTML align attribute for table cells
fn get_align_attr(align: Alignment) -> String {
    match align {
        Alignment::Left => " style=\"text-align: left;\"".to_string(),
        Alignment::Center => " style=\"text-align: center;\"".to_string(),
        Alignment::Right => " style=\"text-align: right;\"".to_string(),
        Alignment::None => String::new(),
    }
}

/// Escape HTML special characters
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Modern CSS styles with sans-serif fonts
const CSS_STYLES: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
                 'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
                 sans-serif;
    line-height: 1.6;
    color: #333;
    background-color: #f5f5f5;
    padding: 20px;
}

.container {
    max-width: 900px;
    margin: 0 auto;
    background: white;
    padding: 60px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    border-radius: 4px;
}

.document-title {
    font-size: 2.5em;
    font-weight: 700;
    margin-bottom: 20px;
    color: #1a1a1a;
    border-bottom: 3px solid #0066cc;
    padding-bottom: 10px;
}

.metadata {
    margin-bottom: 40px;
    padding: 20px;
    background-color: #f9f9f9;
    border-left: 4px solid #0066cc;
    border-radius: 4px;
}

.subtitle {
    font-size: 1.3em;
    color: #555;
    margin-bottom: 10px;
    font-weight: 500;
}

.description {
    font-size: 1.05em;
    color: #666;
    margin-bottom: 20px;
    line-height: 1.5;
}

.metadata-table {
    width: 100%;
    font-size: 0.95em;
    border-collapse: collapse;
}

.metadata-table td {
    padding: 6px 10px;
    border-bottom: 1px solid #eee;
}

.metadata-table td.label {
    font-weight: 600;
    color: #555;
    width: 150px;
}

.section-heading {
    margin-top: 40px;
    margin-bottom: 20px;
    color: #1a1a1a;
    font-weight: 600;
    border-bottom: 2px solid #e0e0e0;
    padding-bottom: 8px;
}

h2.section-heading {
    font-size: 2em;
}

h3.section-heading {
    font-size: 1.6em;
}

h4.section-heading {
    font-size: 1.3em;
}

h5.section-heading {
    font-size: 1.1em;
}

h6.section-heading {
    font-size: 1em;
}

.section-number {
    color: #0066cc;
    margin-right: 8px;
    font-weight: 700;
}

.content-heading {
    margin-top: 24px;
    margin-bottom: 12px;
    color: #333;
    font-weight: 600;
}

p {
    margin-bottom: 16px;
    text-align: justify;
}

strong {
    font-weight: 600;
    color: #1a1a1a;
}

em {
    font-style: italic;
}

del {
    text-decoration: line-through;
    color: #888;
}

code {
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', monospace;
    background-color: #f4f4f4;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.9em;
    color: #d73a49;
}

pre {
    background-color: #f6f8fa;
    border: 1px solid #e1e4e8;
    border-radius: 4px;
    padding: 16px;
    margin-bottom: 16px;
    overflow-x: auto;
}

pre code {
    background: none;
    padding: 0;
    color: #24292e;
    font-size: 0.9em;
    line-height: 1.45;
}

blockquote {
    border-left: 4px solid #ddd;
    padding-left: 16px;
    margin: 16px 0;
    color: #666;
    font-style: italic;
}

ul, ol {
    margin-bottom: 16px;
    padding-left: 30px;
}

li {
    margin-bottom: 8px;
}

ul.task-list {
    list-style: none;
    padding-left: 0;
}

.task-list-item {
    list-style: none;
}

.task-list-item input[type="checkbox"] {
    margin-right: 8px;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 20px;
    font-size: 0.95em;
}

thead {
    background-color: #f6f8fa;
}

th {
    padding: 12px;
    text-align: left;
    font-weight: 600;
    color: #1a1a1a;
    border-bottom: 2px solid #d0d7de;
    border-right: 1px solid #d0d7de;
}

th:last-child {
    border-right: none;
}

td {
    padding: 10px 12px;
    border-bottom: 1px solid #d0d7de;
    border-right: 1px solid #d0d7de;
}

td:last-child {
    border-right: none;
}

tbody tr:hover {
    background-color: #f6f8fa;
}

figure {
    margin: 24px 0;
    text-align: center;
}

figure img {
    max-width: 100%;
    height: auto;
    border: 1px solid #e1e4e8;
    border-radius: 4px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

figcaption {
    margin-top: 8px;
    font-size: 0.9em;
    color: #666;
    font-style: italic;
}

.image-error {
    color: #d73a49;
    background-color: #ffeef0;
    padding: 12px;
    border-radius: 4px;
    border-left: 4px solid #d73a49;
}

hr {
    border: none;
    border-top: 2px solid #e1e4e8;
    margin: 32px 0;
}

a {
    color: #0366d6;
    text-decoration: none;
}

a:hover {
    text-decoration: underline;
}

@media print {
    body {
        background: white;
        padding: 0;
    }

    .container {
        box-shadow: none;
        padding: 0;
    }

    .section-heading {
        page-break-after: avoid;
    }

    figure {
        page-break-inside: avoid;
    }

    table {
        page-break-inside: avoid;
    }
}

@media screen and (max-width: 768px) {
    .container {
        padding: 30px 20px;
    }

    .document-title {
        font-size: 2em;
    }

    table {
        font-size: 0.85em;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_runs_to_html_plain() {
        let runs = vec![TextRun::new("Hello world".to_string())];
        assert_eq!(runs_to_html(&runs), "Hello world");
    }

    #[test]
    fn test_runs_to_html_bold() {
        let mut run = TextRun::new("bold".to_string());
        run.bold = true;
        let runs = vec![run];
        assert_eq!(runs_to_html(&runs), "<strong>bold</strong>");
    }

    #[test]
    fn test_runs_to_html_italic() {
        let mut run = TextRun::new("italic".to_string());
        run.italic = true;
        let runs = vec![run];
        assert_eq!(runs_to_html(&runs), "<em>italic</em>");
    }

    #[test]
    fn test_runs_to_html_code() {
        let mut run = TextRun::new("code".to_string());
        run.code = true;
        let runs = vec![run];
        assert_eq!(runs_to_html(&runs), "<code>code</code>");
    }

    #[test]
    fn test_runs_to_html_link() {
        let mut run = TextRun::new("link text".to_string());
        run.link_url = Some("https://example.com".to_string());
        let runs = vec![run];
        assert_eq!(
            runs_to_html(&runs),
            "<a href=\"https://example.com\">link text</a>"
        );
    }
}
