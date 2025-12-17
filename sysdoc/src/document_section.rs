//! Section model for individual document sections

use pulldown_cmark::{Event, Parser, Tag};
use std::fmt;
use std::path::PathBuf;

/// A section in the document (corresponds to a markdown file)
#[derive(Debug)]
pub struct DocumentSection {
    /// Section number parsed from filename (e.g., [1, 1] from "01.01_purpose.md")
    pub number: SectionNumber,
    /// Section title extracted from filename (e.g., "Purpose" from "01.01_purpose.md")
    pub title: String,
    /// Nesting level (0 = top level, 1 = first subsection, etc.)
    pub depth: usize,
    /// Raw markdown content
    pub content: String,
    /// Parsed markdown events
    pub events: Vec<Event<'static>>,
    /// Image references found in the markdown
    pub images: Vec<ImageReference>,
    /// Table references found in the markdown (CSV files)
    pub tables: Vec<PathBuf>,
    /// Path to source file (for error reporting)
    #[allow(dead_code)]
    pub source_path: PathBuf,
}

impl DocumentSection {
    /// Parse the markdown content and extract references
    pub fn parse_content(&mut self) {
        let parser = Parser::new(&self.content);
        let mut events = Vec::new();
        let mut images: Vec<ImageReference> = Vec::new();
        let mut tables = Vec::new();

        for event in parser {
            Self::process_markdown_event(&event, &mut events, &mut images, &mut tables);
        }

        self.events = events;
        self.images = images;
        self.tables = tables;
    }

    /// Process a single markdown event and extract references
    fn process_markdown_event(
        event: &Event,
        events: &mut Vec<Event<'static>>,
        images: &mut Vec<ImageReference>,
        tables: &mut Vec<PathBuf>,
    ) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                let url = dest_url.to_string();
                images.push(ImageReference {
                    url: url.clone(),
                    alt_text: String::new(), // Will be filled when we see the text
                });
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let url = dest_url.to_string();
                // Check if it's a CSV table reference
                if url.ends_with(".csv") {
                    tables.push(PathBuf::from(url));
                }
            }
            _ => {}
        }
        // Convert to 'static lifetime by cloning strings
        events.push(event.clone().into_static());
    }
}

/// Reference to an image in the markdown
#[derive(Debug, Clone)]
pub struct ImageReference {
    /// URL or path to the image
    #[allow(dead_code)]
    pub url: String,
    /// Alt text for the image
    #[allow(dead_code)]
    pub alt_text: String,
}

/// Section number representation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectionNumber {
    /// Number components (e.g., [1, 2, 3] for "01.02.03")
    pub parts: Vec<u32>,
}

impl SectionNumber {
    /// Parse section number from filename prefix
    /// Examples: "01.01" -> [1, 1], "02.03.01" -> [2, 3, 1]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Option<Vec<u32>> = s.split('.').map(|part| part.parse::<u32>().ok()).collect();

        parts.map(|parts| Self { parts })
    }

    /// Get the depth/nesting level (number of parts - 1)
    pub fn depth(&self) -> usize {
        self.parts.len().saturating_sub(1)
    }
}

impl fmt::Display for SectionNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .parts
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_number_parse() {
        let num = SectionNumber::parse("01.01").unwrap();
        assert_eq!(num.parts, vec![1, 1]);
        assert_eq!(num.depth(), 1);

        let num = SectionNumber::parse("02.03.01").unwrap();
        assert_eq!(num.parts, vec![2, 3, 1]);
        assert_eq!(num.depth(), 2);
    }

    #[test]
    fn test_section_number_ordering() {
        let num1 = SectionNumber::parse("01.01").unwrap();
        let num2 = SectionNumber::parse("01.02").unwrap();
        let num3 = SectionNumber::parse("02.01").unwrap();

        assert!(num1 < num2);
        assert!(num2 < num3);
    }

    #[test]
    fn test_html_comment_vs_html_tag() {
        let markdown = r#"
<!-- This is a comment -->
<div class="test">This is HTML</div>

<!-- sysdoc: id=req-001 -->
"#;

        let parser = Parser::new(markdown);
        let mut comments = Vec::new();
        let mut html_tags = Vec::new();

        for event in parser {
            if let Event::Html(html) = event {
                let html_str = html.to_string();
                if html_str.trim().starts_with("<!--") {
                    comments.push(html_str);
                } else {
                    html_tags.push(html_str);
                }
            }
        }

        // pulldown-cmark does NOT distinguish between comments and HTML
        // Both are returned as Event::Html
        // We need to check the content ourselves to differentiate
        assert_eq!(comments.len(), 2);
        assert!(comments[0].contains("This is a comment"));
        assert!(comments[1].contains("sysdoc: id=req-001"));

        assert_eq!(html_tags.len(), 1);
        assert!(html_tags[0].contains("<div"));
        assert!(html_tags[0].contains("</div>"));
    }

    #[test]
    fn test_fenced_code_block_metadata() {
        let markdown = r#"
# Section Title

```sysdoc
id: req-001
status: approved
priority: high
```

Some content here.

```rust
fn main() {}
```
"#;

        let parser = Parser::new(markdown);
        let mut sysdoc_blocks = Vec::new();

        let mut in_sysdoc_block = false;
        let mut current_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                        if lang.as_ref() == "sysdoc" {
                            in_sysdoc_block = true;
                            current_text.clear();
                        }
                    }
                }
                Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                    if in_sysdoc_block {
                        sysdoc_blocks.push(current_text.clone());
                        in_sysdoc_block = false;
                    }
                }
                Event::Text(text) => {
                    if in_sysdoc_block {
                        current_text.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        assert_eq!(sysdoc_blocks.len(), 1);
        assert!(sysdoc_blocks[0].contains("id: req-001"));
        assert!(sysdoc_blocks[0].contains("status: approved"));
        assert!(sysdoc_blocks[0].contains("priority: high"));
    }
}
