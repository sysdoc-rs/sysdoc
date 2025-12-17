//! Three-stage document processing pipeline
//!
//! This module orchestrates the three stages of document processing:
//! 1. **Parsing**: Load and parse all source files (markdown, images, CSV)
//! 2. **Transformation**: Convert source model into unified document model
//! 3. **Export**: Generate output formats (docx, markdown, etc.)

use crate::document_config::DocumentConfig;
use crate::source_model::{
    ImageFormat, ImageSource, MarkdownBlock, MarkdownSource, SectionNumber, SourceModel,
    TableSource,
};
use crate::unified_document::{
    ContentBlock, DocumentBuilder, DocumentMetadata, DocumentSection, InlineContent, Person,
    UnifiedDocument,
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Stage 1: Parse all source files
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

    // Discover all referenced images
    let image_paths: std::collections::HashSet<_> = model
        .markdown_files
        .iter()
        .flat_map(|md_file| &md_file.sections)
        .flat_map(|section| &section.content)
        .filter_map(|block| {
            if let MarkdownBlock::Image { path, .. } = block {
                Some(path.clone())
            } else {
                None
            }
        })
        .collect();

    // Load image metadata
    for path in image_paths {
        let absolute_path = root.join(&path);
        if absolute_path.exists() {
            let format = ImageFormat::from_path(&path);
            model.image_files.push(ImageSource {
                path,
                absolute_path,
                format,
                loaded: false,
                data: None,
            });
        }
    }

    // Discover all referenced tables
    let mut table_paths = std::collections::HashSet::new();
    for md_file in &model.markdown_files {
        for section in &md_file.sections {
            for table_ref in &section.table_refs {
                table_paths.insert(table_ref.clone());
            }
        }
    }

    // Load table metadata
    for path in table_paths {
        let absolute_path = root.join(&path);
        if absolute_path.exists() {
            model.table_files.push(TableSource {
                path,
                absolute_path,
                loaded: false,
                data: None,
            });
        }
    }

    // Validate all references
    model.validate()?;

    Ok(model)
}

/// Parse a single markdown file
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
    source.parse();

    Ok(source)
}

/// Parse filename into section number and title
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

/// Stage 2: Transform source model into unified document
pub fn transform(source: SourceModel) -> Result<UnifiedDocument, TransformError> {
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
        version: None,
        created: None,
        modified: None,
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

    // Add images
    for image in source.image_files {
        builder.add_image(image);
    }

    // Add tables
    for table in source.table_files {
        builder.add_table(table);
    }

    Ok(builder.build())
}

/// Build hierarchical section structure from flat list
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
    pub fn to_markdown(_doc: &UnifiedDocument, _output_path: &Path) -> Result<(), ExportError> {
        // TODO: Implement markdown export
        Err(ExportError::NotImplemented("Markdown export".to_string()))
    }

    /// Export to Microsoft Word (.docx)
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
#[derive(Debug)]
pub enum ParseError {
    IoError(PathBuf, std::io::Error),
    InvalidFilename(PathBuf),
    InvalidSectionNumber(PathBuf),
    ValidationError(crate::source_model::ValidationError),
    ConfigError(PathBuf, Box<crate::document_config::DocumentConfigError>),
}

impl From<crate::source_model::ValidationError> for ParseError {
    fn from(err: crate::source_model::ValidationError) -> Self {
        ParseError::ValidationError(err)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::IoError(path, e) => {
                write!(f, "IO error reading {}: {}", path.display(), e)
            }
            ParseError::InvalidFilename(path) => {
                write!(f, "Invalid filename format: {}", path.display())
            }
            ParseError::InvalidSectionNumber(path) => {
                write!(f, "Invalid section number in: {}", path.display())
            }
            ParseError::ValidationError(e) => write!(f, "Validation error: {}", e),
            ParseError::ConfigError(path, e) => {
                write!(f, "Config error loading {}: {}", path.display(), e)
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Transformation errors
#[derive(Debug)]
pub enum TransformError {
    InvalidStructure(String),
}

impl std::fmt::Display for TransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformError::InvalidStructure(msg) => {
                write!(f, "Invalid document structure: {}", msg)
            }
        }
    }
}

impl std::error::Error for TransformError {}

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
