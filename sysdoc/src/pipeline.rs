//! Three-stage document processing pipeline
//!
//! This module orchestrates the three stages of document processing:
//! 1. **Parsing**: Load and parse all source files (markdown, images, CSV)
//! 2. **Transformation**: Convert source model into unified document model
//! 3. **Export**: Generate output formats (docx, markdown, etc.)

use crate::document_config::DocumentConfig;
use crate::source_model::{MarkdownSource, SectionNumber, SourceModel};
use crate::unified_document::{
    ContentBlock, DocumentBuilder, DocumentMetadata, DocumentSection, InlineContent, Person,
    UnifiedDocument,
};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Stage 1: Parse all source files
///
/// # Parameters
/// * `root` - Root directory of the document source containing sysdoc.toml and markdown files
///
/// # Returns
/// * `Ok(SourceModel)` - Successfully parsed source model with all discovered files
/// * `Err(ParseError)` - Error loading configuration, parsing files, or validating references
pub fn parse_sources(root: &Path) -> Result<SourceModel, ParseError> {
    // Load document configuration
    let config_path = root.join("sysdoc.toml");
    let config = DocumentConfig::load(&config_path)
        .map_err(|e| ParseError::ConfigError(config_path.clone(), Box::new(e)))?;

    let mut model = SourceModel::new(root.to_path_buf(), config);

    // Discover all markdown files
    let markdown_paths: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("md")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    // Parse markdown files (optionally in parallel)
    #[cfg(feature = "parallel")]
    let markdown_files: Result<Vec<_>, _> = markdown_paths
        .par_iter()
        .map(|path| parse_markdown_file(path, root))
        .collect();

    #[cfg(not(feature = "parallel"))]
    let markdown_files: Result<Vec<_>, _> = markdown_paths
        .iter()
        .map(|path| parse_markdown_file(path, root))
        .collect();

    model.markdown_files = markdown_files?;

    // Note: Images are now embedded directly in MarkdownBlock::Image with metadata
    // resolved during parsing, so we don't need to collect them separately

    // Note: CSV tables are now embedded directly in MarkdownBlock::CsvTable with data
    // loaded during parsing, so we don't need to collect them separately

    // Validate all references
    model.validate()?;

    Ok(model)
}

/// Parse a single markdown file
///
/// # Parameters
/// * `path` - Absolute path to the markdown file to parse
/// * `root` - Root directory of the document (used for calculating relative paths)
///
/// # Returns
/// * `Ok(MarkdownSource)` - Successfully parsed markdown source with content and metadata
/// * `Err(ParseError)` - Error reading file or parsing filename/section number
fn parse_markdown_file(path: &Path, root: &Path) -> Result<MarkdownSource, ParseError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| ParseError::IoError(path.to_path_buf(), e))?;

    // Extract filename without extension
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ParseError::InvalidFilename(path.to_path_buf()))?;

    // Parse section number and title
    let (number_str, title) = parse_filename(filename, path)?;

    let section_number = SectionNumber::parse(number_str)
        .ok_or_else(|| ParseError::InvalidSectionNumber(path.to_path_buf()))?;

    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    let mut source = MarkdownSource {
        path: relative_path,
        absolute_path: path.to_path_buf(),
        section_number,
        title,
        raw_content: content,
        sections: Vec::new(),
    };

    // Parse the markdown content into sections
    source
        .parse(root)
        .map_err(|e| ParseError::SourceModelError(path.to_path_buf(), e))?;

    Ok(source)
}

/// Parse filename into section number and title
///
/// # Parameters
/// * `filename` - Filename without extension (e.g., "01.01_purpose")
/// * `path` - Full path to the file (used for error reporting)
///
/// # Returns
/// * `Ok((number_str, title))` - Successfully parsed section number string and title
/// * `Err(ParseError)` - Invalid filename format (must be "XX.YY_title")
fn parse_filename<'a>(filename: &'a str, path: &Path) -> Result<(&'a str, String), ParseError> {
    let parts: Vec<&str> = filename.splitn(2, '_').collect();

    if parts.len() != 2 {
        return Err(ParseError::InvalidFilename(path.to_path_buf()));
    }

    let number_str = parts[0];
    let title_slug = parts[1];

    // Convert slug to title
    let title = title_slug
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Ok((number_str, title))
}

/// Get the git version using `git describe --tags --dirty`
///
/// # Returns
/// * Git version string if successful
/// * Empty string if git command fails, with a warning logged
fn get_git_version() -> String {
    match std::process::Command::new("git")
        .args(["describe", "--tags", "--dirty"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            log::warn!(
                "Git command failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            String::new()
        }
        Err(e) => {
            log::warn!("Failed to execute git describe: {}", e);
            String::new()
        }
    }
}

/// Get the ISO 8601 datetime of the first git commit
///
/// # Returns
/// * ISO 8601 datetime string if successful
/// * Empty string if git command fails, with a warning logged
fn get_git_first_commit_date() -> String {
    match std::process::Command::new("git")
        .args(["log", "--reverse", "--format=%aI", "--max-count=1"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            log::warn!(
                "Git log command failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            String::new()
        }
        Err(e) => {
            log::warn!("Failed to execute git log for first commit: {}", e);
            String::new()
        }
    }
}

/// Get the ISO 8601 datetime of the current HEAD commit
///
/// # Returns
/// * ISO 8601 datetime string if successful
/// * Empty string if git command fails, with a warning logged
fn get_git_head_commit_date() -> String {
    match std::process::Command::new("git")
        .args(["log", "-1", "--format=%aI", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            log::warn!(
                "Git log command failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            String::new()
        }
        Err(e) => {
            log::warn!("Failed to execute git log for HEAD: {}", e);
            String::new()
        }
    }
}

/// Stage 2: Transform source model into unified document
///
/// # Parameters
/// * `source` - Parsed source model containing all markdown, image, and table files
///
/// # Returns
/// * `Ok(UnifiedDocument)` - Successfully transformed unified document ready for export
/// * `Err(TransformError)` - Error building document structure
pub fn transform(source: SourceModel) -> Result<UnifiedDocument, TransformError> {
    let version_string = get_git_version();
    let version = if version_string.is_empty() {
        None
    } else {
        Some(version_string)
    };

    let created_string = get_git_first_commit_date();
    let created = if created_string.is_empty() {
        None
    } else {
        Some(created_string)
    };

    let modified_string = get_git_head_commit_date();
    let modified = if modified_string.is_empty() {
        None
    } else {
        Some(modified_string)
    };

    let metadata = DocumentMetadata {
        document_id: source.config.document_id.clone(),
        title: source.config.document_name.clone(),
        doc_type: source.config.document_type.clone(),
        standard: source.config.document_standard.clone(),
        template: source.config.document_template.clone(),
        owner: Person {
            name: source.config.document_owner.name.clone(),
            email: source.config.document_owner.email.clone(),
        },
        approver: Person {
            name: source.config.document_approver.name.clone(),
            email: source.config.document_approver.email.clone(),
        },
        version,
        created,
        modified,
    };

    let mut builder = DocumentBuilder::new(metadata, source.root.clone());

    // Sort markdown files by section number
    let mut sorted_files = source.markdown_files;
    sorted_files.sort_by(|a, b| a.section_number.cmp(&b.section_number));

    // Build hierarchical section structure
    let sections = build_section_hierarchy(&sorted_files)?;

    for section in sections {
        builder.add_section(section);
    }

    // Note: Images are now embedded in MarkdownBlock::Image within sections
    // so we don't need to add them separately

    // Note: CSV tables are now embedded in MarkdownBlock::CsvTable within sections
    // so we don't need to add them separately

    Ok(builder.build())
}

/// Build hierarchical section structure from flat list
///
/// # Parameters
/// * `files` - Flat list of markdown source files, assumed to be sorted by section number
///
/// # Returns
/// * `Ok(Vec<DocumentSection>)` - Hierarchical vector of document sections
/// * `Err(TransformError)` - Error building section structure
fn build_section_hierarchy(
    files: &[MarkdownSource],
) -> Result<Vec<DocumentSection>, TransformError> {
    let mut root_sections: Vec<DocumentSection> = Vec::new();

    for file in files {
        let depth = file.section_number.depth();
        let heading_level = depth + 1;

        // For now, create a simple flat structure
        // A more sophisticated implementation would build a proper hierarchy
        let content = if !file.sections.is_empty() {
            // Use the content from the first section
            vec![ContentBlock::Paragraph(vec![InlineContent::Text(
                file.raw_content.clone(),
            )])]
        } else {
            vec![]
        };

        let section = DocumentSection {
            number: file.section_number.clone(),
            title: file.title.clone(),
            depth,
            heading_level,
            content,
            subsections: vec![],
        };

        root_sections.push(section);
    }

    Ok(root_sections)
}

/// Stage 3: Export unified document to various formats
pub mod export {
    use crate::unified_document::UnifiedDocument;
    use std::path::Path;

    /// Export to aggregated markdown file
    ///
    /// # Parameters
    /// * `_doc` - The unified document to export
    /// * `_output_path` - Path where the aggregated markdown file will be written
    ///
    /// # Returns
    /// * `Ok(())` - Successfully exported to markdown
    /// * `Err(ExportError)` - Error during export (currently not implemented)
    pub fn to_markdown(_doc: &UnifiedDocument, _output_path: &Path) -> Result<(), ExportError> {
        // TODO: Implement markdown export
        Err(ExportError::NotImplemented("Markdown export".to_string()))
    }

    /// Export to Microsoft Word (.docx)
    ///
    /// # Parameters
    /// * `_doc` - The unified document to export
    /// * `_output_path` - Path where the .docx file will be written
    ///
    /// # Returns
    /// * `Ok(())` - Successfully exported to DOCX format
    /// * `Err(ExportError)` - Error during export (currently not implemented)
    pub fn to_docx(_doc: &UnifiedDocument, _output_path: &Path) -> Result<(), ExportError> {
        // TODO: Implement docx export
        Err(ExportError::NotImplemented("DOCX export".to_string()))
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
}

/// Parsing errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error reading {path}: {source}", path = .0.display(), source = .1)]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("Invalid filename format: {path}", path = .0.display())]
    InvalidFilename(PathBuf),

    #[error("Invalid section number in: {path}", path = .0.display())]
    InvalidSectionNumber(PathBuf),

    #[error("Validation error: {0}")]
    ValidationError(#[from] crate::source_model::ValidationError),

    #[error("Error parsing {path}: {source}", path = .0.display(), source = .1)]
    SourceModelError(PathBuf, #[source] crate::source_model::SourceModelError),

    #[error("Config error loading {path}: {source}", path = .0.display(), source = .1)]
    ConfigError(
        PathBuf,
        #[source] Box<crate::document_config::DocumentConfigError>,
    ),
}

/// Transformation errors
#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Invalid document structure: {0}")]
    InvalidStructure(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filename() {
        let path = Path::new("test.md");

        let (num, title) = parse_filename("01.01_purpose", path).unwrap();
        assert_eq!(num, "01.01");
        assert_eq!(title, "Purpose");

        let (num, title) = parse_filename("02.03_system_overview", path).unwrap();
        assert_eq!(num, "02.03");
        assert_eq!(title, "System Overview");
    }
}
