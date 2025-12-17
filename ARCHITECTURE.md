# sysdoc Architecture

## Overview

`sysdoc` uses a **three-stage pipeline** to process Systems Engineering documents:

1. **Stage 1: Parsing** - Load and parse all source files
2. **Stage 2: Transformation** - Build unified document model
3. **Stage 3: Export** - Generate output formats

This architecture provides clear separation of concerns and enables future optimizations like caching and incremental builds.

---

## Three-Stage Pipeline

### Stage 1: Parsing (Source Model)

**Goal**: Load all source files from disk, parse their content, and validate references.

**Input**: Directory containing markdown files, images, and CSV tables

**Output**: `SourceModel` containing:
- Parsed markdown files with structured content
- Image file references with metadata
- CSV table references with metadata
- Validation results

**Key Operations**:
```rust
// Parse all source files
let source_model = pipeline::parse_sources(&root_path)?;

// The source model contains:
// - markdown_files: Vec<MarkdownSource>
// - image_files: Vec<ImageSource>
// - table_files: Vec<TableSource>
```

**Parallelization**: This stage is highly parallelizable. Each markdown file can be parsed independently. We use conditional compilation to support both serial and parallel parsing:

```rust
#[cfg(feature = "parallel")]
let markdown_files: Result<Vec<_>, _> = markdown_paths
    .par_iter()
    .map(|path| parse_markdown_file(path, root))
    .collect();

#[cfg(not(feature = "parallel"))]
let markdown_files: Result<Vec<_>, _> = markdown_paths
    .iter()
    .map(|path| parse_markdown_file(path, root))
    .collect();
```

**File Structure**:
- `src/source_model.rs` - Source model data structures
- `src/pipeline.rs` - Parsing implementation

### Stage 2: Transformation (Unified Document Model)

**Goal**: Transform the flat source model into a hierarchical, unified document ready for export.

**Input**: `SourceModel` + `DocumentConfig`

**Output**: `UnifiedDocument` containing:
- Document metadata (title, ID, owner, etc.)
- Hierarchical section structure
- Resolved image and table references
- Adjusted heading levels for nested sections

**Key Operations**:
```rust
// Transform source model into unified document
// Config is already loaded in the source_model during parsing
let document = pipeline::transform(source_model)?;

// The unified document contains:
// - metadata: DocumentMetadata
// - sections: Vec<DocumentSection> (hierarchical)
// - images: Vec<ImageSource>
// - tables: Vec<TableSource>
```

**Transformations**:
1. Sort sections by section number
2. Build hierarchical structure (sections with subsections)
3. Adjust heading levels based on nesting depth
4. Convert markdown events to structured content blocks
5. Resolve image and table references
6. Calculate metadata (word count, etc.)

**File Structure**:
- `src/unified_document.rs` - Unified document model
- `src/pipeline.rs` - Transformation implementation

### Stage 3: Export

**Goal**: Generate output files in various formats (markdown, docx, PDF).

**Input**: `UnifiedDocument`

**Output**: Files on disk (aggregated markdown, .docx, etc.)

**Key Operations**:
```rust
// Export to markdown
pipeline::export::to_markdown(&document, "output/document.md")?;

// Export to docx
pipeline::export::to_docx(&document, "output/document.docx")?;
```

**Supported Formats**:
- Aggregated markdown (`.md` + `images/` folder)
- Microsoft Word (`.docx`)
- PDF (future)

**File Structure**:
- `src/pipeline.rs` - Export module

---

## Data Models

### Source Model (Stage 1)

The source model represents files exactly as they are on disk, with minimal processing.

```rust
pub struct SourceModel {
    pub root: PathBuf,
    pub markdown_files: Vec<MarkdownSource>,
    pub image_files: Vec<ImageSource>,
    pub table_files: Vec<TableSource>,
}

pub struct MarkdownSource {
    pub path: PathBuf,              // Relative path
    pub absolute_path: PathBuf,     // Absolute path
    pub section_number: SectionNumber,
    pub title: String,
    pub raw_content: String,        // Raw markdown
    pub sections: Vec<MarkdownSection>,  // Parsed sections
}

pub struct MarkdownSection {
    pub heading_level: usize,
    pub heading_text: String,
    pub content: Vec<MarkdownContent>,  // Structured content
    pub image_refs: Vec<ImageReference>,
    pub table_refs: Vec<PathBuf>,
}
```

**MarkdownContent Enum**: Captures all pulldown-cmark events in a structured format:
```rust
pub enum MarkdownContent {
    Start(MarkdownTag),
    End(MarkdownTagEnd),
    Text(String),
    Code(String),
    Html(String),
    InlineCode(String),
    SoftBreak,
    HardBreak,
    Rule,
    FootnoteReference(String),
    TaskListMarker(bool),
}
```

This enum preserves all markdown structure for later transformation while being easier to work with than raw pulldown-cmark events.

### Unified Document Model (Stage 2)

The unified document model represents the final, hierarchical document structure.

```rust
pub struct UnifiedDocument {
    pub metadata: DocumentMetadata,
    pub root: PathBuf,
    pub sections: Vec<DocumentSection>,  // Hierarchical
    pub images: Vec<ImageSource>,
    pub tables: Vec<TableSource>,
}

pub struct DocumentSection {
    pub number: SectionNumber,
    pub title: String,
    pub depth: usize,
    pub heading_level: usize,      // Adjusted for nesting
    pub content: Vec<ContentBlock>,
    pub subsections: Vec<DocumentSection>,  // Nested!
}

pub enum ContentBlock {
    Paragraph(Vec<InlineContent>),
    Heading { level: usize, content: Vec<InlineContent> },
    BlockQuote(Vec<ContentBlock>),
    CodeBlock { kind: CodeBlockKind, lang: Option<String>, code: String },
    List { ordered: bool, start: Option<u64>, items: Vec<ListItem> },
    Table { alignments: Vec<Alignment>, headers: Vec<Vec<InlineContent>>, rows: Vec<Vec<Vec<InlineContent>>> },
    CsvTable { path: PathBuf, headers: Vec<String>, rows: Vec<Vec<String>> },
    Image { path: PathBuf, alt_text: String, title: Option<String> },
    Rule,
    Html(String),
}
```

**Key Differences from Source Model**:
- Hierarchical sections (subsections)
- Adjusted heading levels
- Structured content blocks (not raw events)
- Resolved references
- Metadata included

---

## Parallelization

### Is Parallelization Worthwhile?

**Yes, for large documents with many files.**

**Benefits**:
- Markdown parsing is CPU-bound (regex, string processing)
- Files are independent (no shared state during parsing)
- Documents may have 50-200+ markdown files
- Rayon makes it trivial with `.par_iter()`

**When to Use**:
- Documents with 20+ markdown files
- Build servers / CI/CD (faster builds)
- Large teams with complex documents

**When NOT to Use**:
- Small documents (< 10 files) - overhead > benefit
- Limited CPU cores
- Memory-constrained environments

**Implementation**:

We use **conditional compilation** with a feature flag:

```toml
# Cargo.toml
[features]
default = []
parallel = ["rayon"]

[dependencies]
rayon = { version = "1.8", optional = true }
```

```rust
// pipeline.rs
#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[cfg(feature = "parallel")]
let markdown_files: Result<Vec<_>, _> = markdown_paths
    .par_iter()
    .map(|path| parse_markdown_file(path, root))
    .collect();

#[cfg(not(feature = "parallel"))]
let markdown_files: Result<Vec<_>, _> = markdown_paths
    .iter()
    .map(|path| parse_markdown_file(path, root))
    .collect();
```

**Usage**:
```bash
# Build without parallelization (default)
cargo build --release

# Build with parallelization
cargo build --release --features parallel

# Benchmark both
hyperfine 'sysdoc build' 'sysdoc build --features parallel'
```

**Benchmarking**: Use `criterion` or `hyperfine` to measure the actual benefit on representative documents.

---

## Image and CSV Reference Handling

### Discovery and Validation

**Stage 1 (Parsing)**:
1. Parse markdown files
2. Extract image references from `![alt](path)` syntax
3. Extract CSV references from `[table](path.csv)` syntax
4. Discover actual files on disk
5. **Validate**: Check that all references exist

```rust
impl SourceModel {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check all image references exist
        // Check all table references exist
        // Return detailed errors with file locations
    }
}
```

**Stage 2 (Transformation)**:
1. Load image metadata (format, dimensions)
2. Optionally load image data into memory
3. Parse CSV files into structured data
4. Include in unified document

**Stage 3 (Export)**:
1. Copy images to output directory
2. Embed images in docx
3. Convert CSV data to tables

### Loading Strategy

**Option 1: Lazy Loading** (Current)
```rust
pub struct ImageSource {
    pub path: PathBuf,
    pub loaded: bool,
    pub data: Option<Vec<u8>>,  // Load on demand
}

impl ImageSource {
    pub fn load(&mut self) -> std::io::Result<()> {
        self.data = Some(std::fs::read(&self.absolute_path)?);
        self.loaded = true;
        Ok(())
    }
}
```

**Option 2: Eager Loading**
```rust
// Load all images during parsing
for mut image in source_model.image_files {
    image.load()?;  // Load immediately
}
```

**Recommendation**: Use lazy loading by default, add `--preload-images` flag for eager loading.

---

## Logging

### Should You Use `env_logger` for a CLI?

**Yes! It's excellent practice for Rust CLI applications.**

**Benefits**:
1. **Zero cost when disabled** - No overhead in release builds
2. **Flexible verbosity** - Users control via `RUST_LOG` environment variable
3. **Standard practice** - Expected by Rust developers
4. **Debugging** - Invaluable during development
5. **Production troubleshooting** - Users can enable logging when reporting bugs

**Recommended Setup**:

```toml
# Cargo.toml
[dependencies]
log = "0.4"
env_logger = "0.11"
```

```rust
// main.rs
use log::{debug, info, warn, error};

fn main() {
    // Initialize logger (respects RUST_LOG environment variable)
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)  // Default level
        .init();

    info!("sysdoc starting...");
    debug!("Debug information");

    // Your code here
}
```

**Usage**:
```bash
# Default (info level)
sysdoc build

# Enable debug logging
RUST_LOG=debug sysdoc build

# Enable trace logging for specific module
RUST_LOG=sysdoc::pipeline=trace sysdoc build

# Disable all logging
RUST_LOG=off sysdoc build
```

**When to Log**:
- `trace!()` - Very detailed, per-file operations
- `debug!()` - Useful debugging info (parsing progress, file counts)
- `info!()` - Important user-facing info (build started, completed)
- `warn!()` - Warnings that don't stop execution (missing optional files)
- `error!()` - Errors before returning `Err`

**Example**:
```rust
pub fn parse_sources(root: &Path) -> Result<SourceModel, ParseError> {
    info!("Parsing sources from: {}", root.display());

    let markdown_paths = discover_markdown_files(root);
    debug!("Found {} markdown files", markdown_paths.len());

    for path in &markdown_paths {
        trace!("Parsing: {}", path.display());
        // ... parse file ...
    }

    info!("Parsing complete: {} files, {} images, {} tables",
          model.markdown_files.len(),
          model.image_files.len(),
          model.table_files.len());

    Ok(model)
}
```

**Alternative**: `tracing` crate (more advanced, structured logging)
- Better for async code
- Spans and events
- More overhead

**Recommendation**: Start with `env_logger`, migrate to `tracing` if you need structured logging or async support.

---

## Migration Path

### From Current Code to Three-Stage Pipeline

**Phase 1: Create New Models** (✓ Done)
- [x] Create `source_model.rs`
- [x] Create `unified_document.rs`
- [x] Create `pipeline.rs`

**Phase 2: Implement Parsing**
- [ ] Implement `pipeline::parse_sources()`
- [ ] Add tests for markdown parsing
- [ ] Add tests for image/CSV discovery
- [ ] Add validation

**Phase 3: Implement Transformation**
- [ ] Implement `pipeline::transform()`
- [ ] Build section hierarchy
- [ ] Adjust heading levels
- [ ] Add tests

**Phase 4: Implement Export**
- [ ] Implement markdown export
- [ ] Implement docx export
- [ ] Add tests

**Phase 5: Integration**
- [ ] Update `main.rs` to use pipeline
- [ ] Update CLI commands
- [ ] Add logging throughout
- [ ] Update documentation

**Phase 6: Optimization**
- [ ] Add parallelization feature
- [ ] Benchmark performance
- [ ] Add lazy loading for images
- [ ] Consider caching between builds

---

## File Organization

```
sysdoc/src/
├── main.rs                 # CLI entry point
├── cli.rs                  # CLI argument parsing
├── pipeline.rs             # Three-stage pipeline orchestration
├── source_model.rs         # Stage 1: Source model (NEW)
├── unified_document.rs     # Stage 2: Document model (NEW)
├── document_config.rs      # Document configuration (sysdoc.toml)
├── template_config.rs      # Template configuration
├── templates.rs            # Embedded templates
├── document_section.rs     # Legacy (to be removed)
├── document_model.rs       # Legacy (to be removed)
└── walker.rs              # Legacy (functionality moved to pipeline)
```

---

## Design Decisions

### Why Three Stages?

**Separation of Concerns**:
- Parsing: I/O and validation
- Transformation: Business logic
- Export: Format-specific rendering

**Benefits**:
- Easier testing (test each stage independently)
- Clearer error messages (know which stage failed)
- Future optimizations (cache parsed sources, skip transformation if unchanged)
- Multiple export formats from same unified model

### Why Two Models (Source + Unified)?

**Source Model**: Represents files as-is
- Minimal processing
- Easy to cache
- Matches disk structure

**Unified Model**: Represents final document
- Hierarchical structure
- Resolved references
- Export-ready

**Alternative**: Single model
- Pro: Less code
- Con: Harder to cache, harder to test, mixing concerns

### Why Enum for Markdown Content?

**Alternatives Considered**:

1. **Keep raw `pulldown_cmark::Event`** - Hard to work with, lifetime issues
2. **Convert to HTML** - Loses structure, hard to manipulate
3. **Custom enum** (chosen) - Structured, owned, easy to pattern match

**Benefits**:
- No lifetime issues (all owned data)
- Easy to pattern match
- Preserves full markdown structure
- Can be serialized (for caching)

---

## Future Enhancements

1. **Incremental Builds**
   - Cache parsed sources
   - Only re-parse changed files
   - Checksum-based invalidation

2. **Watch Mode**
   - Watch for file changes
   - Automatically rebuild
   - Live preview in browser

3. **Parallel Export**
   - Generate multiple formats in parallel
   - docx + markdown + PDF simultaneously

4. **Language Server Protocol (LSP)**
   - IDE integration
   - Real-time validation
   - Jump to definition (for references)

5. **Plugins**
   - Custom transformations
   - Custom export formats
   - Custom validation rules

6. **Differential Rendering**
   - Show changes between versions
   - Track requirement changes
   - Generate changelog

---

## Performance Considerations

### Memory Usage

**Lazy Loading** (recommended):
- Load images only when needed for export
- Stream CSV parsing for large tables
- Release markdown source after transformation

**Memory Budget** (rough estimates):
- Small doc (10 files, 10 images): ~5 MB
- Medium doc (50 files, 50 images): ~25 MB
- Large doc (200 files, 200 images): ~100 MB

### CPU Usage

**Bottlenecks**:
1. Markdown parsing (regex, string processing)
2. Image loading (decompression)
3. CSV parsing
4. Docx generation (XML generation, zipping)

**Optimization Opportunities**:
- Parallel markdown parsing (rayon)
- Parallel image loading
- Stream processing for CSV
- Incremental docx generation

### Disk I/O

**Minimize Reads**:
- Read each file once
- Cache in memory during processing
- Batch writes for export

---

## Testing Strategy

### Unit Tests
- Parse individual markdown files
- Transform individual sections
- Export individual content blocks

### Integration Tests
- Full pipeline (parse → transform → export)
- Real template documents
- Validate output format

### Benchmark Tests
- Parse performance (small, medium, large docs)
- Transform performance
- Export performance
- Memory usage

---

## Error Handling

### Error Types by Stage

**Stage 1 (Parsing)**:
```rust
pub enum ParseError {
    IoError(PathBuf, std::io::Error),
    InvalidFilename(PathBuf),
    InvalidSectionNumber(PathBuf),
    ValidationError(ValidationError),
}
```

**Stage 2 (Transformation)**:
```rust
pub enum TransformError {
    InvalidStructure(String),
}
```

**Stage 3 (Export)**:
```rust
pub enum ExportError {
    IoError(std::io::Error),
    FormatError(String),
    NotImplemented(String),
}
```

### Error Messages

**Good Error Messages**:
```
Error: Missing image 'diagrams/architecture.png' referenced in 'src/03_design/03.01_overview.md'

Hint: Check that the image file exists at the correct path relative to the document root.
```

**Poor Error Messages**:
```
Error: File not found
```

---

## Summary

The three-stage pipeline provides:
- Clear separation of concerns
- Easy testing and debugging
- Future optimization opportunities
- Multiple export formats from one source

The new models provide:
- Structured markdown content (enum)
- Image and CSV validation
- Hierarchical document structure
- Export-ready format

Using `env_logger` is highly recommended for CLI applications.

Parallelization is worthwhile for documents with many files (20+), but should be optional via feature flag.
