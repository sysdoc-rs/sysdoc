//! Section number representation

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

impl std::fmt::Display for SectionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .parts
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", s)
    }
}
