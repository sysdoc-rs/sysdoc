//! Command-line interface definitions for sysdoc

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Output format for the build command
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Microsoft Word DOCX format
    Docx,
    /// Markdown with images in a separate folder
    Markdown,
    /// HTML with embedded images
    Html,
    /// PDF with embedded images and table of contents
    Pdf,
}

/// CLI structure for the sysdoc application
#[derive(Parser)]
#[command(name = "sysdoc")]
#[command(version)]
#[command(about = "Systems Engineering documentation tool", long_about = None)]
pub struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for sysdoc
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new document from a DID template
    Init {
        /// DID template identifier (e.g., DI-IPSC-81435B, SDD, IDD)
        template: String,

        /// Directory to initialize (defaults to current directory)
        path: Option<PathBuf>,

        /// Overwrite existing files
        #[arg(short, long)]
        force: bool,

        /// Document title
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Build documentation to .docx or markdown format
    Build {
        /// Input directory (defaults to current directory)
        #[arg(value_name = "PATH", default_value = ".")]
        input: PathBuf,

        /// Output file or directory path
        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        /// Output format (auto-detected from file extension if not specified)
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,

        /// Watch for changes and rebuild automatically
        #[arg(short, long)]
        watch: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Skip table of contents generation
        #[arg(long)]
        no_toc: bool,

        /// Skip image embedding (DOCX only)
        #[arg(long)]
        no_images: bool,
    },

    /// Validate document structure and references
    Validate {
        /// Input directory (defaults to current directory)
        #[arg(value_name = "PATH", default_value = ".")]
        input: PathBuf,

        /// Show detailed validation results
        #[arg(short, long)]
        verbose: bool,

        /// Validate all internal references
        #[arg(long)]
        check_links: bool,

        /// Verify all image files exist
        #[arg(long)]
        check_images: bool,

        /// Validate CSV table references
        #[arg(long)]
        check_tables: bool,
    },

    /// List available DID templates
    ListTemplates,
}
