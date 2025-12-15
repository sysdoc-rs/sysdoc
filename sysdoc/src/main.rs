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

mod cli;
mod document_config;
mod document_model;
mod document_section;
mod template_config;
mod templates;
mod walker;

use clap::Parser;
use cli::{Cli, Commands, OutputFormat};
use walker::walk_document;

/// Main entry point for the sysdoc CLI application
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            template,
            path,
            force,
            title,
        } => {
            let target_path = path.unwrap_or_else(|| std::path::PathBuf::from("."));

            // Look up the template
            let template_info = match templates::get_template(&template) {
                Some(t) => t,
                None => {
                    eprintln!("Error: Template '{}' not found", template);
                    eprintln!("\nRun 'sysdoc list-templates' to see available templates");
                    std::process::exit(1);
                }
            };

            // Parse the template configuration
            let config = match templates::parse_template(&template_info) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error parsing template '{}': {}", template, e);
                    std::process::exit(1);
                }
            };

            println!("Initializing {} document from template: {}", config.document_type, config.name);
            println!("Target path: {}", target_path.display());
            if let Some(ref title_text) = title {
                println!("Title: {}", title_text);
            }

            // Create the target directory if it doesn't exist
            if !target_path.exists() {
                if let Err(e) = std::fs::create_dir_all(&target_path) {
                    eprintln!("Error creating directory {}: {}", target_path.display(), e);
                    std::process::exit(1);
                }
            }

            // Check if directory is empty (unless force flag is set)
            if !force {
                if let Ok(entries) = std::fs::read_dir(&target_path) {
                    if entries.count() > 0 {
                        eprintln!("Error: Target directory is not empty");
                        eprintln!("Use --force to overwrite existing files");
                        std::process::exit(1);
                    }
                }
            }

            // Create all files from the template
            let mut files_created = 0;
            for file_path in config.files.keys() {
                let full_path = target_path.join(file_path);

                // Create parent directories if needed
                if let Some(parent) = full_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("Error creating directory {}: {}", parent.display(), e);
                        std::process::exit(1);
                    }
                }

                // Generate file content
                let mut content = match config.generate_file_content(file_path) {
                    Some(c) => c,
                    None => {
                        eprintln!("Error: Could not generate content for {}", file_path);
                        continue;
                    }
                };

                // Replace title placeholder if provided
                if let Some(ref title_text) = title {
                    content = content.replace("{{TITLE}}", title_text);
                }

                // Write the file
                if let Err(e) = std::fs::write(&full_path, content) {
                    eprintln!("Error writing file {}: {}", full_path.display(), e);
                    std::process::exit(1);
                }

                files_created += 1;
            }

            println!("\n✓ Successfully created {} files", files_created);
            println!("\nNext steps:");
            println!("  1. Edit sysdoc.toml to configure your document");
            println!("  2. Fill in the markdown files in the src/ directory");
            println!("  3. Run 'sysdoc build' to generate the output document");
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
            if verbose {
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

            // Walk the document directory and build the model
            match walk_document(&input) {
                Ok(document) => {
                    if verbose {
                        println!("\nDiscovered {} sections:", document.sections.len());
                        for section in &document.sections {
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
                    }

                    println!("\nDocument parsed successfully!");
                    println!("Sections: {}", document.sections.len());

                    // Summary of resources
                    let total_images: usize =
                        document.sections.iter().map(|s| s.images.len()).sum();
                    let total_tables: usize =
                        document.sections.iter().map(|s| s.tables.len()).sum();
                    if total_images > 0 || total_tables > 0 {
                        println!(
                            "Resources: {} images, {} tables",
                            total_images, total_tables
                        );
                    }

                    // TODO: Implement rendering to DOCX or Markdown
                    if watch {
                        println!("Watch mode not yet implemented");
                    }
                }
                Err(e) => {
                    eprintln!("Error building document: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Validate {
            input,
            verbose,
            check_links,
            check_images,
            check_tables,
        } => {
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
        }

        Commands::ListTemplates => {
            println!("Available DID templates:\n");

            for template in templates::get_all_templates() {
                println!("  {} - {} ({})", template.spec, template.doc_type, template.id);
                println!("    Aliases: {}, {}", template.doc_type, template.id);
                println!();
            }

            println!("Usage: sysdoc init <template> [path]");
            println!("Example: sysdoc init SDD ./my-document");
        }
    }
}
