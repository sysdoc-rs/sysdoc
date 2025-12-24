//! Generates a minimal DOCX template for testing
//!
//! Run with: cargo run --example generate_test_template

use docx_rust::document::Paragraph;
use docx_rust::Docx;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let template_path = Path::new("tests/fixtures/template.docx");

    // Ensure parent directory exists
    if let Some(parent) = template_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create a minimal DOCX document
    let mut docx = Docx::default();

    // Add a placeholder paragraph (will be replaced by sysdoc content)
    docx.document.push(Paragraph::default());

    // Write the template
    docx.write_file(template_path)
        .map_err(|e| format!("Failed to write template: {}", e))?;

    println!("Created template at: {}", template_path.display());
    Ok(())
}
