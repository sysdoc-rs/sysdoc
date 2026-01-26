//! sysdoc - Systems Engineering documentation tool
//!
//! A CLI tool for creating and building Systems Engineering documents
//! using Markdown, DrawIO, and CSV files.

#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::all))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::pedantic))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(missing_docs))]
// Allow some pedantic lints that are too strict for this project
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_variant_names)]
// Until 1.0.0, allow dead code warnings
#![allow(dead_code)]

mod cli;
mod document_config;
mod document_model;
mod document_section;
mod template_config;
mod templates;
mod walker;

// New three-stage pipeline modules
mod pipeline;
mod source_model;
mod unified_document;

// DOCX exporter (template-preserving)
mod docx_template_exporter;

// Markdown exporter
mod markdown_exporter;

// HTML exporter
mod html_exporter;

// PDF exporter (Typst-based)
mod typst_exporter;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, OutputFormat};

/// Main entry point for the sysdoc CLI application
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

/// Run the CLI application
fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            template,
            path,
            force,
            title,
        } => {
            handle_init_command(template, path, force, title)?;
        }

        Commands::Build {
            input,
            output,
            format,
            verbose,
            no_toc: _,
            no_images,
        } => {
            handle_build_command(input, output, format, verbose, no_images)?;
        }

        Commands::Validate {
            input,
            verbose,
            check_images,
            check_tables,
        } => {
            handle_validate_command(input, verbose, check_images, check_tables)?;
        }

        Commands::ListTemplates => {
            handle_list_templates_command();
        }
    }

    Ok(())
}

/// Handle the init command
fn handle_init_command(
    template: String,
    path: Option<std::path::PathBuf>,
    force: bool,
    title: Option<String>,
) -> Result<()> {
    let target_path = path.unwrap_or_else(|| std::path::PathBuf::from("."));

    // Look up the template
    let template_info = templates::get_template(&template).with_context(|| {
        format!(
            "Template '{}' not found. Run 'sysdoc list-templates' to see available templates",
            template
        )
    })?;

    // Parse the template configuration
    let config = templates::parse_template(&template_info)
        .with_context(|| format!("Failed to parse template '{}'", template))?;

    println!(
        "Initializing {} document from template: {}",
        config.document_type, config.name
    );
    println!("Target path: {}", target_path.display());
    if let Some(ref title_text) = title {
        println!("Title: {}", title_text);
    }

    // Create the target directory if it doesn't exist
    if !target_path.exists() {
        std::fs::create_dir_all(&target_path)
            .with_context(|| format!("Failed to create directory {}", target_path.display()))?;
    }

    // Check if directory is empty (unless force flag is set)
    if !force {
        check_directory_empty(&target_path)?;
    }

    // Create all files from the template
    let files_created = create_template_files(&config, &target_path, &title)?;

    // Create binary files from the template
    let binary_files_created = create_binary_files(&template_info, &target_path)?;

    println!(
        "\n✓ Successfully created {} files",
        files_created + binary_files_created
    );
    println!("\nNext steps:");
    println!("  1. Edit sysdoc.toml to configure your document");
    println!("  2. Fill in the markdown files in the src/ directory");
    println!("  3. Run 'sysdoc build' to generate the output document");

    Ok(())
}

/// Handle the build command
fn handle_build_command(
    input: std::path::PathBuf,
    mut output: std::path::PathBuf,
    format_arg: Option<OutputFormat>,
    verbose: bool,
    no_images: bool,
) -> Result<()> {
    // Canonicalize input path to get absolute path with drive letter on Windows
    let input = input
        .canonicalize()
        .with_context(|| format!("Failed to resolve input path: {}", input.display()))?;

    // Auto-detect format from output file extension if not explicitly specified
    let format = match format_arg {
        Some(fmt) => {
            // Format explicitly specified, add appropriate extension if missing
            if output.extension().is_none() {
                let ext = match fmt {
                    OutputFormat::Docx => "docx",
                    OutputFormat::Markdown => "md",
                    OutputFormat::Html => "html",
                    OutputFormat::Pdf => "pdf",
                };
                output.set_extension(ext);
            }
            fmt
        }
        None => {
            // Auto-detect from file extension
            match output.extension().and_then(|s| s.to_str()) {
                Some("docx") => OutputFormat::Docx,
                Some("md") | Some("markdown") => OutputFormat::Markdown,
                Some("html") | Some("htm") => OutputFormat::Html,
                Some("pdf") => OutputFormat::Pdf,
                Some(ext) => {
                    anyhow::bail!(
                        "Unknown output format for extension '.{}'. Supported: .docx, .md, .html, .pdf\nUse --format to specify explicitly.",
                        ext
                    );
                }
                None => {
                    // No extension, default to PDF
                    output.set_extension("pdf");
                    OutputFormat::Pdf
                }
            }
        }
    };

    // Initialize logging if verbose
    if verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
        print_build_info(&input, &output, format, no_images);
    }

    println!("Building documentation...");
    println!("Input: {}", input.display());
    println!("Output: {}", output.display());

    // Stage 1: Parse all source files (includes validation)
    println!("\n[Stage 1/3] Parsing source files...");
    let source_model = match pipeline::parse_sources(&input) {
        Ok(model) => model,
        Err(e) => {
            eprintln!("✗ Validation failed:\n");
            eprintln!("{}", format_parse_error(&e));
            anyhow::bail!("Build failed due to validation errors");
        }
    };
    println!(
        "✓ Parsed {} markdown files, validation passed",
        source_model.markdown_files.len()
    );

    // Extract template path from config before consuming source_model
    let docx_template_path = source_model
        .config
        .docx_template_path
        .as_ref()
        .map(|p| input.join(p));

    // Stage 2: Transform to unified document
    println!("\n[Stage 2/3] Transforming to unified document...");
    let unified_doc = pipeline::transform(source_model)
        .with_context(|| "Failed to transform source model to unified document")?;

    println!("✓ Transformed {} sections", unified_doc.sections.len());
    if verbose {
        println!("  - {} words", unified_doc.word_count());
        println!("  - {} images", unified_doc.image_count());
        println!("  - {} tables", unified_doc.table_count());
    }

    // Stage 3: Export to output format
    println!(
        "\n[Stage 3/3] Exporting to {}...",
        match format {
            OutputFormat::Docx => "DOCX",
            OutputFormat::Markdown => "Markdown",
            OutputFormat::Html => "HTML",
            OutputFormat::Pdf => "PDF",
        }
    );

    match format {
        OutputFormat::Docx => {
            let template_path = docx_template_path.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "DOCX export requires a template. Set 'docx_template_path' in sysdoc.toml"
                )
            })?;
            docx_template_exporter::to_docx(&unified_doc, template_path, &output)
                .with_context(|| format!("Failed to export DOCX to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
        OutputFormat::Markdown => {
            pipeline::export::to_markdown(&unified_doc, &output)
                .with_context(|| format!("Failed to export Markdown to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
        OutputFormat::Html => {
            pipeline::export::to_html(&unified_doc, &output)
                .with_context(|| format!("Failed to export HTML to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
        OutputFormat::Pdf => {
            typst_exporter::to_pdf(&unified_doc, &output)
                .with_context(|| format!("Failed to export PDF to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
    }

    println!("\n✓ Build completed successfully!");

    Ok(())
}

/// Handle the validate command
///
/// Validates the document structure and references. By default, all validation
/// checks are run. Use `--check-images` or `--check-tables` to run only specific checks.
///
/// # Parameters
/// * `input` - Path to the document directory containing sysdoc.toml
/// * `verbose` - Show detailed validation output
/// * `check_images` - Only check image references
/// * `check_tables` - Only check CSV table references
///
/// # Returns
/// * `Ok(())` - All validation checks passed
/// * `Err` - Validation failed with error details
fn handle_validate_command(
    input: std::path::PathBuf,
    verbose: bool,
    check_images: bool,
    check_tables: bool,
) -> Result<()> {
    // Determine if we're running selective checks or all checks
    let selective_mode = check_images || check_tables;

    if verbose {
        println!("Validating document at: {}", input.display());
        if selective_mode {
            if check_images {
                println!("  - Checking image references");
            }
            if check_tables {
                println!("  - Checking table references");
            }
        } else {
            println!("  - Running all validation checks");
        }
    }

    // Check that sysdoc.toml exists
    let config_path = input.join("sysdoc.toml");
    if !config_path.exists() {
        anyhow::bail!(
            "No sysdoc.toml found in '{}'. Is this a sysdoc project directory?",
            input.display()
        );
    }

    // Parse sources - this runs validation internally
    // Note: validation is all-or-nothing. The --check-images and --check-tables flags
    // are kept for backwards compatibility but don't filter which validations run.
    match pipeline::parse_sources(&input) {
        Ok(model) => {
            // All checks passed (parse_sources would have failed if validation failed)
            if verbose {
                let section_count = count_sections(&model);
                let image_count = count_images(&model);
                let table_count = count_tables(&model);
                println!("  Found {} sections", section_count);
                println!("  Found {} image references", image_count);
                println!("  Found {} table references", table_count);
            }
            println!("✓ Validation passed");
            Ok(())
        }
        Err(e) => {
            // Validation failed - format the error nicely
            eprintln!("✗ Validation failed:\n");
            eprintln!("{}", format_parse_error(&e));
            anyhow::bail!("Validation failed")
        }
    }
}

/// Count the number of image references in the model
fn count_images(model: &source_model::SourceModel) -> usize {
    model
        .markdown_files
        .iter()
        .flat_map(|f| f.sections.iter())
        .flat_map(|s| s.content.iter())
        .filter(|block| matches!(block, source_model::MarkdownBlock::Image { .. }))
        .count()
}

/// Count the number of table references in the model
fn count_tables(model: &source_model::SourceModel) -> usize {
    model
        .markdown_files
        .iter()
        .flat_map(|f| f.sections.iter())
        .flat_map(|s| s.content.iter())
        .filter(|block| matches!(block, source_model::MarkdownBlock::CsvTable { .. }))
        .count()
}

/// Count the number of sections in the model
fn count_sections(model: &source_model::SourceModel) -> usize {
    model.markdown_files.iter().map(|f| f.sections.len()).sum()
}

/// Format a parse error for user-friendly display
fn format_parse_error(error: &pipeline::ParseError) -> String {
    match error {
        pipeline::ParseError::ValidationError(ve) => format_validation_error(ve),
        _ => format!("  {}", error),
    }
}

/// Format a validation error for user-friendly display
fn format_validation_error(error: &source_model::ValidationError) -> String {
    use source_model::ValidationError;

    match error {
        ValidationError::Multiple(errors) => {
            let mut output = String::new();
            for e in errors {
                output.push_str(&format!("  - {}\n", e));
            }
            output
        }
        _ => format!("  - {}", error),
    }
}

/// Handle the list-templates command
fn handle_list_templates_command() {
    println!("Available DID templates:\n");

    for template in templates::get_all_templates() {
        println!(
            "  {} - {} ({})",
            template.spec, template.doc_type, template.id
        );
        println!("    Aliases: {}, {}", template.doc_type, template.id);
        println!();
    }

    println!("Usage: sysdoc init <template> [path]");
    println!("Example: sysdoc init SDD ./my-document");
}

/// Check if a directory is empty
fn check_directory_empty(path: &std::path::Path) -> Result<()> {
    if let Ok(entries) = std::fs::read_dir(path) {
        if entries.count() > 0 {
            anyhow::bail!("Target directory is not empty. Use --force to overwrite existing files");
        }
    }
    Ok(())
}

/// Create all files from the template
fn create_template_files(
    config: &template_config::TemplateConfig,
    target_path: &std::path::Path,
    title: &Option<String>,
) -> Result<usize> {
    let mut files_created = 0;

    for file_path in config.files.keys() {
        create_single_file(config, target_path, file_path, title)
            .with_context(|| format!("Failed to create file {}", file_path))?;
        files_created += 1;
    }

    Ok(files_created)
}

/// Create a single file from the template
fn create_single_file(
    config: &template_config::TemplateConfig,
    target_path: &std::path::Path,
    file_path: &str,
    title: &Option<String>,
) -> Result<()> {
    let full_path = target_path.join(file_path);

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Generate file content
    let mut content = config
        .generate_file_content(file_path)
        .with_context(|| format!("Could not generate content for {}", file_path))?;

    // Replace title placeholder if provided
    if let Some(ref title_text) = title {
        content = content.replace("{{TITLE}}", title_text);
    }

    // Write the file
    std::fs::write(&full_path, content)
        .with_context(|| format!("Failed to write file {}", full_path.display()))?;

    Ok(())
}

/// Create binary files from the template
fn create_binary_files(
    template_info: &templates::TemplateInfo,
    target_path: &std::path::Path,
) -> Result<usize> {
    let mut files_created = 0;

    for (file_name, content) in &template_info.binary_files {
        let full_path = target_path.join(file_name);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        std::fs::write(&full_path, content)
            .with_context(|| format!("Failed to write binary file {}", full_path.display()))?;

        files_created += 1;
    }

    Ok(files_created)
}

/// Print build information
fn print_build_info(
    input: &std::path::Path,
    output: &std::path::Path,
    format: OutputFormat,
    no_images: bool,
) {
    println!("Building documentation...");
    println!("Input: {}", input.display());
    println!("Output: {}", output.display());
    match format {
        OutputFormat::Docx => println!("Format: DOCX"),
        OutputFormat::Markdown => {
            println!("Format: Markdown with images folder");
            if no_images {
                println!("Warning: --no-images has no effect in Markdown format");
            }
        }
        OutputFormat::Html => {
            println!("Format: HTML with embedded images");
            if no_images {
                println!("Warning: --no-images has no effect in HTML format");
            }
        }
        OutputFormat::Pdf => {
            println!("Format: PDF with embedded images and table of contents");
            if no_images {
                println!("Warning: --no-images has no effect in PDF format");
            }
        }
    }
}

/// Print section summary
fn print_section_summary(document: &document_model::DocumentModel) {
    println!("\nDiscovered {} sections:", document.sections.len());
    for section in &document.sections {
        print_section_info(section);
    }
}

/// Print information about a single section
fn print_section_info(section: &document_section::DocumentSection) {
    println!(
        "  {} - {} (depth: {}, {} chars, {} events, {} images, {} tables)",
        section.number,
        section.title,
        section.depth,
        section.content.len(),
        section.events.len(),
        section.images.len(),
        section.tables.len()
    );
}
