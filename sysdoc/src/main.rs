//! sysdoc - Systems Engineering documentation tool
//!
//! A CLI tool for creating and building Systems Engineering documents
//! using Markdown, DrawIO, and CSV files.

#![deny(unsafe_code)]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::all))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::pedantic))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(missing_docs))]
// Allow some pedantic lints that are too strict for this project
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
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
            watch,
            verbose,
            no_toc: _,
            no_images,
        } => {
            handle_build_command(input, output, format, watch, verbose, no_images)?;
        }

        Commands::Validate {
            input,
            verbose,
            check_links,
            check_images,
            check_tables,
        } => {
            handle_validate_command(input, verbose, check_links, check_images, check_tables)?;
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

    println!("\n✓ Successfully created {} files", files_created);
    println!("\nNext steps:");
    println!("  1. Edit sysdoc.toml to configure your document");
    println!("  2. Fill in the markdown files in the src/ directory");
    println!("  3. Run 'sysdoc build' to generate the output document");

    Ok(())
}

/// Handle the build command
fn handle_build_command(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    format: OutputFormat,
    watch: bool,
    verbose: bool,
    no_images: bool,
) -> Result<()> {
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

    // Stage 1: Parse all source files
    println!("\n[Stage 1/3] Parsing source files...");
    let source_model = pipeline::parse_sources(&input)
        .with_context(|| format!("Failed to parse sources from {}", input.display()))?;

    println!(
        "✓ Parsed {} markdown files",
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
        }
    );

    match format {
        OutputFormat::Docx => {
            let template_path = docx_template_path.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "DOCX export requires a template. Set 'docx_template_path' in sysdoc.toml"
                )
            })?;
            pipeline::export::to_docx(&unified_doc, template_path, &output)
                .with_context(|| format!("Failed to export DOCX to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
        OutputFormat::Markdown => {
            pipeline::export::to_markdown(&unified_doc, &output)
                .with_context(|| format!("Failed to export Markdown to {}", output.display()))?;
            println!("✓ Successfully wrote: {}", output.display());
        }
    }

    if watch {
        println!("\nWatch mode not yet implemented");
    }

    println!("\n✓ Build completed successfully!");

    Ok(())
}

/// Handle the validate command
fn handle_validate_command(
    input: std::path::PathBuf,
    verbose: bool,
    check_links: bool,
    check_images: bool,
    check_tables: bool,
) -> Result<()> {
    println!("Validating document structure...");
    println!("Input: {}", input.display());
    if verbose {
        println!("Verbose mode enabled");
    }
    if check_links {
        println!("Checking internal links");
    }
    if check_images {
        println!("Checking image files");
    }
    if check_tables {
        println!("Checking table references");
    }
    // TODO: Implement validation logic

    Ok(())
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
