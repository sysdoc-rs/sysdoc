//! sysdoc - Systems Engineering documentation tool
//!
//! A CLI tool for creating and building Systems Engineering documents
//! using Markdown, DrawIO, and CSV files.

#![deny(unsafe_code)]
#![cfg_attr(
    all(not(debug_assertions), not(test)),
    deny(clippy::all, missing_docs, unused_crate_dependencies)
)]

mod cli;

use clap::Parser;
use cli::{Cli, Commands, OutputFormat};

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
            no_toc,
            no_images,
        } => {
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
            if watch {
                println!("Watch mode enabled");
            }
            if verbose {
                println!("Verbose mode enabled");
            }
            if no_toc {
                println!("Skipping table of contents");
            }
            if no_images && matches!(format, OutputFormat::Docx) {
                println!("Skipping image embedding");
            }
            // TODO: Implement build logic
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
