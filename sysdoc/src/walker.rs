//! Directory walker for discovering markdown files in document structure

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::document_model::DocumentModel;
use crate::document_section::{DocumentSection, SectionNumber};

/// Errors that can occur during document walking
#[derive(Debug)]
pub enum WalkerError {
    /// IO error
    Io(std::io::Error),
    /// Invalid filename format
    InvalidFilename(PathBuf),
    /// Missing section number in filename
    MissingSectionNumber(PathBuf),
}

impl From<std::io::Error> for WalkerError {
    fn from(err: std::io::Error) -> Self {
        WalkerError::Io(err)
    }
}

impl std::fmt::Display for WalkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalkerError::Io(e) => write!(f, "IO error: {}", e),
            WalkerError::InvalidFilename(path) => {
                write!(f, "Invalid filename format: {}", path.display())
            }
            WalkerError::MissingSectionNumber(path) => {
                write!(f, "Missing section number in filename: {}", path.display())
            }
        }
    }
}

impl std::error::Error for WalkerError {}

/// Walk a document directory and build the document model
pub fn walk_document(root: &Path) -> Result<DocumentModel, WalkerError> {
    let mut document = DocumentModel::new(root.to_path_buf());
    let mut sections = Vec::new();

    // Walk the directory tree
    for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = entry.map_err(std::io::Error::other)?;
        let path = entry.path();

        // Only process markdown files
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        // Parse the section from the file
        // Skip files that don't follow the section numbering convention (e.g., README.md)
        match parse_section(path, root) {
            Ok(section) => sections.push(section),
            Err(WalkerError::InvalidFilename(_)) | Err(WalkerError::MissingSectionNumber(_)) => {
                // Skip files that don't match the section format
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // Sort sections by their section number
    sections.sort_by(|a, b| a.number.cmp(&b.number));

    document.sections = sections;
    Ok(document)
}

/// Parse a single markdown file into a Section
fn parse_section(path: &Path, _root: &Path) -> Result<DocumentSection, WalkerError> {
    // Read file content
    let content = fs::read_to_string(path)?;

    // Extract filename without extension
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| WalkerError::InvalidFilename(path.to_path_buf()))?;

    // Parse section number and title from filename
    // Expected format: "XX.YY_title" or "XX.YY.ZZ_title"
    let (number_str, title) = parse_filename(filename, path)?;

    let number = SectionNumber::parse(number_str)
        .ok_or_else(|| WalkerError::MissingSectionNumber(path.to_path_buf()))?;

    let depth = number.depth();

    let mut section = DocumentSection {
        number,
        title: title.to_string(),
        depth,
        content,
        events: Vec::new(),
        images: Vec::new(),
        tables: Vec::new(),
        source_path: path.to_path_buf(),
    };

    // Parse the markdown content to extract events and references
    section.parse_content();

    Ok(section)
}

/// Parse filename into section number and title
/// Examples:
///   "01.01_purpose" -> ("01.01", "Purpose")
///   "02.03.01_details" -> ("02.03.01", "Details")
fn parse_filename<'a>(filename: &'a str, path: &Path) -> Result<(&'a str, String), WalkerError> {
    // Find the underscore separator
    let parts: Vec<&str> = filename.splitn(2, '_').collect();

    if parts.len() != 2 {
        return Err(WalkerError::InvalidFilename(path.to_path_buf()));
    }

    let number_str = parts[0];
    let title_slug = parts[1];

    // Convert slug to title (replace hyphens/underscores with spaces, capitalize)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filename() {
        let path = Path::new("test.md");

        let (num, title) = parse_filename("01.01_purpose", &path).unwrap();
        assert_eq!(num, "01.01");
        assert_eq!(title, "Purpose");

        let (num, title) = parse_filename("02.03_system-overview", &path).unwrap();
        assert_eq!(num, "02.03");
        assert_eq!(title, "System Overview");

        let (num, title) = parse_filename("03.01.02_detailed-design", &path).unwrap();
        assert_eq!(num, "03.01.02");
        assert_eq!(title, "Detailed Design");
    }
}
