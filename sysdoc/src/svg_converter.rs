//! SVG to PNG conversion for DOCX export compatibility
//!
//! This module provides SVG to PNG conversion for formats that don't
//! support SVG natively or have limited SVG 2.0 support, such as
//! Microsoft Word (.docx).

use thiserror::Error;

/// Default DPI for SVG to PNG conversion (150 for better print quality)
pub const DEFAULT_CONVERSION_DPI: f32 = 150.0;

/// SVG conversion errors
#[derive(Error, Debug)]
pub enum SvgConversionError {
    #[error("Failed to parse SVG: {0}")]
    ParseError(String),

    #[error("Failed to create image buffer: dimensions {width}x{height}")]
    BufferError { width: u32, height: u32 },

    #[error("Failed to encode PNG")]
    EncodeError,
}

/// Conversion result containing PNG bytes and dimensions
pub struct ConversionResult {
    /// PNG image bytes
    pub png_bytes: Vec<u8>,
    /// Width in pixels
    pub width_px: u32,
    /// Height in pixels
    pub height_px: u32,
}

/// Convert SVG bytes to PNG bytes
///
/// # Parameters
/// * `svg_data` - Raw SVG file bytes
/// * `dpi` - Optional DPI for conversion (defaults to 150)
///
/// # Returns
/// * `Ok(ConversionResult)` - PNG bytes and dimensions
/// * `Err(SvgConversionError)` - Conversion failed
pub fn svg_to_png(
    svg_data: &[u8],
    dpi: Option<f32>,
) -> Result<ConversionResult, SvgConversionError> {
    let dpi = dpi.unwrap_or(DEFAULT_CONVERSION_DPI);

    // Parse SVG using usvg
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_data, &options)
        .map_err(|e| SvgConversionError::ParseError(e.to_string()))?;

    // Calculate pixel dimensions from SVG viewBox
    // SVG default is 96 DPI (CSS pixels), scale to target DPI
    let size = tree.size();
    let scale = dpi / 96.0;
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;

    // Ensure dimensions are valid
    if width == 0 || height == 0 {
        return Err(SvgConversionError::BufferError { width, height });
    }

    // Create pixel buffer
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or(SvgConversionError::BufferError { width, height })?;

    // Render SVG to pixel buffer
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Encode as PNG
    let png_bytes = pixmap
        .encode_png()
        .map_err(|_| SvgConversionError::EncodeError)?;

    Ok(ConversionResult {
        png_bytes,
        width_px: width,
        height_px: height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_svg_conversion() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="red"/>
        </svg>"#;

        let result = svg_to_png(svg, Some(96.0)).unwrap();
        assert_eq!(result.width_px, 100);
        assert_eq!(result.height_px, 100);
        assert!(!result.png_bytes.is_empty());
        // Verify PNG magic bytes
        assert_eq!(&result.png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_svg_with_viewbox() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect width="200" height="100" fill="blue"/>
        </svg>"#;

        let result = svg_to_png(svg, Some(96.0)).unwrap();
        assert_eq!(result.width_px, 200);
        assert_eq!(result.height_px, 100);
    }

    #[test]
    fn test_invalid_svg_returns_error() {
        let invalid = b"not valid svg";
        let result = svg_to_png(invalid, None);
        assert!(result.is_err());
        assert!(matches!(result, Err(SvgConversionError::ParseError(_))));
    }

    #[test]
    fn test_dpi_scaling() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="green"/>
        </svg>"#;

        let result_96 = svg_to_png(svg, Some(96.0)).unwrap();
        let result_192 = svg_to_png(svg, Some(192.0)).unwrap();

        assert_eq!(result_96.width_px, 100);
        assert_eq!(result_192.width_px, 200); // 2x DPI = 2x pixels
    }

    #[test]
    fn test_default_dpi() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96">
            <rect width="96" height="96" fill="yellow"/>
        </svg>"#;

        let result = svg_to_png(svg, None).unwrap();
        // Default 150 DPI: 96 * (150/96) = 150
        assert_eq!(result.width_px, 150);
        assert_eq!(result.height_px, 150);
    }
}
