//! Image source and reference types

use std::path::PathBuf;

/// Reference to an image file (used in markdown content)
#[derive(Debug, Clone)]
pub struct ImageReference {
    /// Path to the image file (relative to document root)
    pub path: PathBuf,

    /// Alt text for the image
    pub alt_text: String,
}

/// An image source file
#[derive(Debug, Clone)]
pub struct ImageSource {
    /// Path to the image file (relative to document root)
    pub path: PathBuf,

    /// Absolute path to the image file
    pub absolute_path: PathBuf,

    /// Image format (png, jpg, svg, etc.)
    pub format: ImageFormat,

    /// Whether the image has been loaded into memory
    pub loaded: bool,

    /// Image data (if loaded)
    pub data: Option<Vec<u8>>,
}

impl ImageSource {
    /// Load the image data into memory
    pub fn load(&mut self) -> std::io::Result<()> {
        self.data = Some(std::fs::read(&self.absolute_path)?);
        self.loaded = true;
        Ok(())
    }
}

/// Image format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Svg,
    DrawIoSvg, // Special handling for .drawio.svg files
    Other,
}

impl ImageFormat {
    /// Determine format from file extension
    pub fn from_path(path: &std::path::Path) -> Self {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check for .drawio.svg first
        if path.to_string_lossy().ends_with(".drawio.svg") {
            return ImageFormat::DrawIoSvg;
        }

        match extension.as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "svg" => ImageFormat::Svg,
            _ => ImageFormat::Other,
        }
    }
}
