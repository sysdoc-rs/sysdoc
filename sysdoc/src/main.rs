//! sysdoc - Systems Engineering documentation tool
//!
//! A CLI tool for creating and building Systems Engineering documents
//! using Markdown, DrawIO, and CSV files.

#![deny(unsafe_code)]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::all))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(missing_docs))]

mod cli;
mod document_config;
mod document_model;
mod document_section;
mod template_config;
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
            println!("Initializing document from template: {}", template);
            println!("Target path: {}", target_path.display());
            if let Some(title) = title {
                println!("Title: {}", title);
            }
            if force {
                println!("Force mode: will overwrite existing files");
            }
            // TODO: Implement init logic
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
            println!("Available DID templates:");
            println!("  DI-IPSC-81435B - Software Design Description (SDD)");
            println!("  SDD            - Software Design Description (alias)");
            // TODO: Load templates from actual template directory
        }
    }
}
