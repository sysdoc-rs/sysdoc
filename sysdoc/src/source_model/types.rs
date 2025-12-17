//! Shared type definitions

/// Table cell alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

impl From<pulldown_cmark::Alignment> for Alignment {
    fn from(align: pulldown_cmark::Alignment) -> Self {
        match align {
            pulldown_cmark::Alignment::None => Alignment::None,
            pulldown_cmark::Alignment::Left => Alignment::Left,
            pulldown_cmark::Alignment::Center => Alignment::Center,
            pulldown_cmark::Alignment::Right => Alignment::Right,
        }
    }
}
