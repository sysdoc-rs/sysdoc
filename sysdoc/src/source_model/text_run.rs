//! Text run representation with formatting
//!
//! A text run is a span of text with consistent formatting applied.
//! This is the fundamental unit for rendering formatted text in DOCX.

/// A span of text with consistent formatting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    /// The text content
    pub text: String,

    /// Bold formatting
    pub bold: bool,

    /// Italic formatting
    pub italic: bool,

    /// Inline code formatting
    pub code: bool,

    /// Strikethrough formatting
    pub strikethrough: bool,

    /// Superscript formatting
    pub superscript: bool,

    /// Subscript formatting
    pub subscript: bool,

    /// Link URL (if this text is part of a hyperlink)
    pub link_url: Option<String>,

    /// Link title (if this text is part of a hyperlink)
    pub link_title: Option<String>,
}

impl TextRun {
    /// Create a new plain text run
    pub fn new(text: String) -> Self {
        Self {
            text,
            bold: false,
            italic: false,
            code: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            link_url: None,
            link_title: None,
        }
    }

    /// Create a new text run with the specified formatting
    pub fn with_formatting(text: String, formatting: &TextFormatting) -> Self {
        Self {
            text,
            bold: formatting.bold,
            italic: formatting.italic,
            code: formatting.code,
            strikethrough: formatting.strikethrough,
            superscript: formatting.superscript,
            subscript: formatting.subscript,
            link_url: formatting.link_url.clone(),
            link_title: formatting.link_title.clone(),
        }
    }

    /// Check if this text run has any formatting applied
    pub fn has_formatting(&self) -> bool {
        self.bold
            || self.italic
            || self.code
            || self.strikethrough
            || self.superscript
            || self.subscript
            || self.link_url.is_some()
    }
}

/// Active formatting state during parsing
///
/// This is used as a stack to track which formatting is currently active
/// as we process the markdown event stream.
#[derive(Debug, Clone, Default)]
pub struct TextFormatting {
    /// Bold formatting active
    pub bold: bool,

    /// Italic formatting active
    pub italic: bool,

    /// Inline code formatting active
    pub code: bool,

    /// Strikethrough formatting active
    pub strikethrough: bool,

    /// Superscript formatting active
    pub superscript: bool,

    /// Subscript formatting active
    pub subscript: bool,

    /// Link URL (if inside a link)
    pub link_url: Option<String>,

    /// Link title (if inside a link)
    pub link_title: Option<String>,
}

impl TextFormatting {
    /// Create a new empty formatting state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any formatting is active
    pub fn has_formatting(&self) -> bool {
        self.bold
            || self.italic
            || self.code
            || self.strikethrough
            || self.superscript
            || self.subscript
            || self.link_url.is_some()
    }
}
