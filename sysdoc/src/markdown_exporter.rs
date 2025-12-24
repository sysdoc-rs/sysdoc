//! Markdown exporter for aggregated documents
//!
//! This module exports a UnifiedDocument to a single markdown file with:
//! - Numbered headings (using section numbers like 1.2.3)
//! - Images embedded as data URLs (base64 encoded)

use crate::source_model::{Alignment, ListItem, MarkdownBlock, MarkdownSection, TextRun};
use crate::unified_document::UnifiedDocument;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during markdown export
#[derive(Error, Debug)]
pub enum MarkdownExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image read error for {path}: {source}", path = .path.display())]
    ImageReadError {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Export a unified document to markdown format
///
/// # Parameters
/// * `doc` - The unified document to export
/// * `output_path` - Path where the markdown file will be written
///
/// # Returns
/// * `Ok(())` - Successfully exported to markdown
/// * `Err(MarkdownExportError)` - Error during export
pub fn to_markdown(doc: &UnifiedDocument, output_path: &Path) -> Result<(), MarkdownExportError> {
    let mut output = String::new();

    // Write document title as H1 if available
    if !doc.metadata.title.is_empty() {
        output.push_str(&format!("# {}\n\n", doc.metadata.title));
    }

    // Write each section
    for section in &doc.sections {
        write_section(&mut output, section)?;
    }

    // Write to file
    let mut file = fs::File::create(output_path)?;
    file.write_all(output.as_bytes())?;

    Ok(())
}

/// Write a single section to the output
fn write_section(
    output: &mut String,
    section: &MarkdownSection,
) -> Result<(), MarkdownExportError> {
    // Write heading with section number
    let heading_prefix = "#".repeat(section.heading_level.min(6));
    output.push_str(&format!(
        "{} {} {}\n\n",
        heading_prefix, section.section_number, section.heading_text
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
) -> Result<(), MarkdownExportError> {
    let indent = "  ".repeat(indent_level);

    match block {
        MarkdownBlock::Heading { level, runs } => {
            // Note: Headings within content are rare (usually only top-level sections have headings)
            let prefix = "#".repeat((*level).min(6));
            output.push_str(&format!(
                "{}{} {}\n\n",
                indent,
                prefix,
                runs_to_markdown(runs)
            ));
        }

        MarkdownBlock::Paragraph(runs) => {
            output.push_str(&indent);
            output.push_str(&runs_to_markdown(runs));
            output.push_str("\n\n");
        }

        MarkdownBlock::Image {
            absolute_path,
            alt_text,
            title,
            format,
            exists,
            ..
        } => {
            write_image(
                output,
                absolute_path,
                alt_text,
                title,
                format,
                *exists,
                &indent,
            );
        }

        MarkdownBlock::CodeBlock {
            language,
            code,
            fenced,
        } => {
            if *fenced {
                let lang = language.as_deref().unwrap_or("");
                output.push_str(&format!("{}```{}\n", indent, lang));
                for line in code.lines() {
                    output.push_str(&indent);
                    output.push_str(line);
                    output.push('\n');
                }
                output.push_str(&format!("{}```\n\n", indent));
            } else {
                // Indented code block
                for line in code.lines() {
                    output.push_str(&indent);
                    output.push_str("    ");
                    output.push_str(line);
                    output.push('\n');
                }
                output.push('\n');
            }
        }

        MarkdownBlock::BlockQuote(blocks) => {
            for inner_block in blocks {
                let mut block_output = String::new();
                write_block(&mut block_output, inner_block, 0)?;
                for line in block_output.lines() {
                    output.push_str(&format!("{}> {}\n", indent, line));
                }
            }
            output.push('\n');
        }

        MarkdownBlock::List { start, items } => {
            write_list(output, start, items, &indent)?;
            output.push('\n');
        }

        MarkdownBlock::InlineTable {
            alignments,
            headers,
            rows,
        } => {
            write_inline_table(output, alignments, headers, rows, &indent);
        }

        MarkdownBlock::CsvTable { data, .. } => {
            if let Some(table_data) = data {
                write_csv_table(output, table_data, &indent);
            }
        }

        MarkdownBlock::Rule => {
            output.push_str(&format!("{}---\n\n", indent));
        }

        MarkdownBlock::Html(html) => {
            output.push_str(&indent);
            output.push_str(html);
            output.push_str("\n\n");
        }
    }

    Ok(())
}

/// Write an image block to markdown output
fn write_image(
    output: &mut String,
    absolute_path: &Path,
    alt_text: &str,
    title: &str,
    format: &crate::source_model::ImageFormat,
    exists: bool,
    indent: &str,
) {
    if !exists {
        output.push_str(&format!(
            "{}![{}](<!-- Image not found: {} -->)\n\n",
            indent,
            alt_text,
            absolute_path.display()
        ));
        return;
    }

    let data = match fs::read(absolute_path) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Failed to read image {}: {}", absolute_path.display(), e);
            output.push_str(&format!(
                "{}![{}](<!-- Image not found: {} -->)\n\n",
                indent,
                alt_text,
                absolute_path.display()
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
        output.push_str(&format!("{}![{}]({})\n\n", indent, alt_text, data_url));
    } else {
        output.push_str(&format!(
            "{}![{}]({} \"{}\")\n\n",
            indent, alt_text, data_url, title
        ));
    }
}

/// Convert text runs to markdown string with formatting
fn runs_to_markdown(runs: &[TextRun]) -> String {
    let mut result = String::new();

    for run in runs {
        let mut text = run.text.clone();

        // Apply formatting in order: code, bold, italic, strikethrough
        if run.code {
            text = format!("`{}`", text);
        }
        if run.bold {
            text = format!("**{}**", text);
        }
        if run.italic {
            text = format!("*{}*", text);
        }
        if run.strikethrough {
            text = format!("~~{}~~", text);
        }
        if run.superscript {
            text = format!("<sup>{}</sup>", text);
        }
        if run.subscript {
            text = format!("<sub>{}</sub>", text);
        }

        // Apply link if present
        if let Some(ref url) = run.link_url {
            if let Some(ref title) = run.link_title {
                text = format!("[{}]({} \"{}\")", text, url, title);
            } else {
                text = format!("[{}]({})", text, url);
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
    indent: &str,
) -> Result<(), MarkdownExportError> {
    for (i, item) in items.iter().enumerate() {
        let marker = get_list_marker(start, item, i);
        write_list_item(output, item, &marker, indent)?;
    }

    Ok(())
}

/// Get the marker string for a list item
fn get_list_marker(start: &Option<u64>, item: &ListItem, index: usize) -> String {
    if let Some(start_num) = start {
        format!("{}. ", start_num + index as u64)
    } else if let Some(checked) = item.task_list {
        if checked {
            "- [x] ".to_string()
        } else {
            "- [ ] ".to_string()
        }
    } else {
        "- ".to_string()
    }
}

/// Write a single list item to output
fn write_list_item(
    output: &mut String,
    item: &ListItem,
    marker: &str,
    indent: &str,
) -> Result<(), MarkdownExportError> {
    let nested_indent_level = indent.len() / 2 + 1;

    let mut blocks = item.content.iter();
    if let Some(first_block) = blocks.next() {
        write_first_list_block(output, first_block, marker, indent, nested_indent_level)?;
    }

    for block in blocks {
        write_block(output, block, nested_indent_level)?;
    }

    Ok(())
}

/// Write the first block of a list item (inline with marker)
fn write_first_list_block(
    output: &mut String,
    block: &MarkdownBlock,
    marker: &str,
    indent: &str,
    nested_indent_level: usize,
) -> Result<(), MarkdownExportError> {
    if let MarkdownBlock::Paragraph(runs) = block {
        output.push_str(&format!("{}{}{}\n", indent, marker, runs_to_markdown(runs)));
    } else {
        output.push_str(&format!("{}{}\n", indent, marker));
        write_block(output, block, nested_indent_level)?;
    }
    Ok(())
}

/// Write an inline markdown table
fn write_inline_table(
    output: &mut String,
    alignments: &[Alignment],
    headers: &[Vec<TextRun>],
    rows: &[Vec<Vec<TextRun>>],
    indent: &str,
) {
    // Write header row
    output.push_str(indent);
    output.push('|');
    for header in headers {
        output.push_str(&format!(" {} |", runs_to_markdown(header)));
    }
    output.push('\n');

    // Write separator row with alignment
    output.push_str(indent);
    output.push('|');
    for (i, _header) in headers.iter().enumerate() {
        let align = alignments.get(i).copied().unwrap_or(Alignment::None);
        let sep = match align {
            Alignment::Left => ":---",
            Alignment::Center => ":---:",
            Alignment::Right => "---:",
            Alignment::None => "---",
        };
        output.push_str(&format!(" {} |", sep));
    }
    output.push('\n');

    // Write data rows
    for row in rows {
        output.push_str(indent);
        output.push('|');
        for cell in row {
            output.push_str(&format!(" {} |", runs_to_markdown(cell)));
        }
        output.push('\n');
    }

    output.push('\n');
}

/// Write a CSV table as a markdown table
fn write_csv_table(output: &mut String, data: &[Vec<String>], indent: &str) {
    if data.is_empty() {
        return;
    }

    // First row is headers
    let headers = &data[0];

    // Write header row
    output.push_str(indent);
    output.push('|');
    for header in headers {
        output.push_str(&format!(" {} |", header));
    }
    output.push('\n');

    // Write separator row
    output.push_str(indent);
    output.push('|');
    for _ in headers {
        output.push_str(" --- |");
    }
    output.push('\n');

    // Write data rows
    for row in data.iter().skip(1) {
        output.push_str(indent);
        output.push('|');
        for cell in row {
            output.push_str(&format!(" {} |", cell));
        }
        output.push('\n');
    }

    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runs_to_markdown_plain() {
        let runs = vec![TextRun::new("Hello world".to_string())];
        assert_eq!(runs_to_markdown(&runs), "Hello world");
    }

    #[test]
    fn test_runs_to_markdown_bold() {
        let mut run = TextRun::new("bold".to_string());
        run.bold = true;
        let runs = vec![run];
        assert_eq!(runs_to_markdown(&runs), "**bold**");
    }

    #[test]
    fn test_runs_to_markdown_italic() {
        let mut run = TextRun::new("italic".to_string());
        run.italic = true;
        let runs = vec![run];
        assert_eq!(runs_to_markdown(&runs), "*italic*");
    }

    #[test]
    fn test_runs_to_markdown_code() {
        let mut run = TextRun::new("code".to_string());
        run.code = true;
        let runs = vec![run];
        assert_eq!(runs_to_markdown(&runs), "`code`");
    }

    #[test]
    fn test_runs_to_markdown_link() {
        let mut run = TextRun::new("link text".to_string());
        run.link_url = Some("https://example.com".to_string());
        let runs = vec![run];
        assert_eq!(runs_to_markdown(&runs), "[link text](https://example.com)");
    }

    #[test]
    fn test_runs_to_markdown_combined() {
        let mut run = TextRun::new("text".to_string());
        run.bold = true;
        run.italic = true;
        let runs = vec![run];
        assert_eq!(runs_to_markdown(&runs), "***text***");
    }
}
