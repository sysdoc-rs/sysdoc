# Pulldown-cmark Event Stream Behavior

This document summarizes the actual behavior of pulldown_cmark's event stream based on unit test observations.

## Key Findings

### 1. Images Are ALWAYS Wrapped in Paragraphs

**Finding:** Standalone images are automatically wrapped in paragraph events by pulldown_cmark.

```rust
// Input markdown:
"![alt text](image.png)"

// Event stream:
Event::Start(Tag::Paragraph)
Event::Start(Tag::Image { ... })
Event::Text("alt text")
Event::End(Image)
Event::End(Paragraph)
```

**Implication:** Images are never top-level block elements in the event stream. They always appear within a paragraph context.

### 2. Top-Level Block Elements

The following elements appear as **top-level blocks** (NOT wrapped in paragraphs):

- **Headings** (`# Heading`)
- **Block Quotes** (`> Quote`)
- **Code Blocks** (` ```code``` `)
- **Lists** (`- item` or `1. item`)
- **Tables** (requires tables extension)
- **Horizontal Rules** (`---`, `***`, `___`)
- **HTML Blocks** (`<div>...</div>`)

### 3. Always in Paragraphs

The following elements are ALWAYS wrapped in paragraph events:

- Plain text
- Images (`![alt](image.png)`)
- Links (`[text](url)`)
- Inline code (`` `code` ``)
- Formatted text (`**bold**`, `*italic*`)

### 4. Images with Text

When images appear with text, everything is in a single paragraph:

```rust
// Input:
"Some text ![alt](image.png) more text"

// Structure:
Paragraph {
    Text("Some text ")
    Image { ... }
    Text(" more text")
}
```

### 5. Multiple Images

Multiple images in one line stay in the same paragraph:

```rust
// Input:
"![img1](a.png) text ![img2](b.png) more ![img3](c.png)"

// All within one Paragraph block with 3 Image tags
```

### 6. Images in Lists

Images in list items follow the same paragraph wrapping rule:

```rust
// Input:
- ![image in list](img.png)

// Structure:
List
  Item
    Image { ... }  // Still wrapped in implicit paragraph
```

## Implications for Our Parser

### Current Behavior

Our parser extracts images from the event stream and creates separate `MarkdownBlock::Image` blocks. This is visible in the test output:

```rust
// We create:
Image { path: "inline.png", alt_text: "...", title: "" }
Paragraph([TextRun { text: "Some text...", ... }])
```

### Design Decision

We have two options:

1. **Keep images as separate blocks** (current behavior)
   - Easier to work with for document generation
   - Images can be rendered independently
   - Loses the exact inline positioning information

2. **Keep images within paragraphs** (more faithful to markdown)
   - Preserves inline structure
   - More complex to render
   - Better represents "image in middle of text" scenarios

### Recommendation

For the **Stage 2 Transform** (unified document model), we should consider:

- Keeping track of whether an image is "standalone" (only content in paragraph) vs "inline" (mixed with text)
- The unified model's `InlineContent::Image` variant suggests we already plan for inline images
- Our `MarkdownBlock::Image` might be for standalone images only

## Test Coverage

The unit tests in [parser.rs](../sysdoc/src/source_model/parser.rs) verify:

✅ Standalone images are wrapped in paragraphs
✅ Images with text stay in one paragraph
✅ Top-level block elements identified
✅ Paragraph wrapping behavior documented
✅ Multiple images in one paragraph
✅ Images in lists
✅ Our parser's image extraction behavior

## Event Stream Summary

```
Top-Level (NOT in paragraphs):
├── Heading
├── BlockQuote
├── CodeBlock
├── List
│   └── Item (can contain paragraphs, images, etc.)
├── Table (with tables extension)
├── Rule
└── HTML Block

Always in Paragraphs:
├── Plain Text
├── Images ⚠️
├── Links
├── Inline Code
└── Formatted Text (bold, italic, etc.)
```

## References

- Tests: [sysdoc/src/source_model/parser.rs](../sysdoc/src/source_model/parser.rs#L591-L844)
- pulldown-cmark: https://github.com/raphlinus/pulldown-cmark
- CommonMark Spec: https://spec.commonmark.org/
