//! Section number representation

use thiserror::Error;

/// Maximum allowed depth for section numbers
pub const MAX_SECTION_DEPTH: usize = 6;

/// Section number representation with fixed maximum depth
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionNumber {
    /// Number components (e.g., [1, 2, 3, 0, 0, 0] with len=3 for "01.02.03")
    parts: [u32; MAX_SECTION_DEPTH],
    /// Number of valid parts (1-6)
    len: usize,
}

/// Errors that can occur when working with section numbers
#[derive(Error, Debug)]
pub enum SectionNumberError {
    #[error("Section number depth exceeds maximum of {MAX_SECTION_DEPTH}: attempted depth {0}")]
    DepthExceeded(usize),

    #[error("Invalid section number format: {0}")]
    InvalidFormat(String),
}

impl SectionNumber {
    /// Parse section number from filename prefix
    ///
    /// # Parameters
    /// * `s` - String slice containing the section number (e.g., "01.01", "02.03.01")
    ///
    /// # Returns
    /// * `Some(SectionNumber)` - Successfully parsed section number
    /// * `None` - Failed to parse (invalid format or non-numeric parts)
    ///
    /// # Examples
    /// * "01.01" -> [1, 1]
    /// * "02.03.01" -> [2, 3, 1]
    pub fn parse(s: &str) -> Option<Self> {
        let parsed_parts: Vec<u32> = s
            .split('.')
            .map(|part| part.parse::<u32>().ok())
            .collect::<Option<Vec<_>>>()?;

        if parsed_parts.is_empty() || parsed_parts.len() > MAX_SECTION_DEPTH {
            return None;
        }

        let mut parts = [0u32; MAX_SECTION_DEPTH];
        parts[..parsed_parts.len()].copy_from_slice(&parsed_parts);

        Some(Self {
            parts,
            len: parsed_parts.len(),
        })
    }

    /// Create a new section number by extending this one with additional parts
    ///
    /// # Parameters
    /// * `additional` - Additional section numbers to append
    ///
    /// # Returns
    /// * `Ok(SectionNumber)` - Successfully extended section number
    /// * `Err(SectionNumberError)` - Depth would exceed maximum
    ///
    /// # Examples
    /// * [1, 2].extend(&[3, 4]) -> [1, 2, 3, 4]
    pub fn extend(&self, additional: &[u32]) -> Result<Self, SectionNumberError> {
        let new_len = self.len + additional.len();

        if new_len > MAX_SECTION_DEPTH {
            return Err(SectionNumberError::DepthExceeded(new_len));
        }

        let mut parts = self.parts;
        parts[self.len..new_len].copy_from_slice(additional);

        Ok(Self {
            parts,
            len: new_len,
        })
    }

    /// Check if this section number ends with .00 (parent section marker)
    ///
    /// # Returns
    /// * `true` if the last part is 0, `false` otherwise
    pub fn is_parent_marker(&self) -> bool {
        self.len > 0 && self.parts[self.len - 1] == 0
    }

    /// Get the parent section number (remove .00 suffix if present)
    ///
    /// # Returns
    /// * `Some(SectionNumber)` - Parent section number if this ends in .00
    /// * `None` - If this doesn't end in .00 or has only one part
    pub fn without_parent_marker(&self) -> Option<Self> {
        if self.is_parent_marker() && self.len > 1 {
            Some(Self {
                parts: self.parts,
                len: self.len - 1,
            })
        } else {
            None
        }
    }

    /// Get the depth/nesting level (number of parts - 1)
    ///
    /// # Returns
    /// * `usize` - The nesting depth (0 for top-level sections, 1 for first subsection, etc.)
    pub fn depth(&self) -> usize {
        self.len.saturating_sub(1)
    }

    /// Calculate the effective heading level for output based on section number depth
    ///
    /// The section number already encodes the full hierarchy (e.g., "3.1.2.1" for an h2
    /// under section 3.1.2), so the effective level is simply depth + 1.
    ///
    /// # Returns
    /// * `usize` - The effective heading level for output, clamped to 1-6
    pub fn effective_heading_level(&self) -> usize {
        (self.depth() + 1).clamp(1, 6)
    }

    /// Get a slice of the valid parts
    ///
    /// # Returns
    /// * `&[u32]` - Slice containing only the valid parts
    pub fn parts(&self) -> &[u32] {
        &self.parts[..self.len]
    }
}

impl PartialOrd for SectionNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.parts().cmp(other.parts())
    }
}

impl std::fmt::Display for SectionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .parts()
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
    fn test_parse() {
        let num = SectionNumber::parse("01.02.03").unwrap();
        assert_eq!(num.parts(), &[1, 2, 3]);
        assert_eq!(num.len, 3);
        assert_eq!(num.to_string(), "1.2.3");
    }

    #[test]
    fn test_parse_max_depth() {
        let num = SectionNumber::parse("01.02.03.04.05.06").unwrap();
        assert_eq!(num.parts(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(num.len, 6);
    }

    #[test]
    fn test_parse_exceeds_max_depth() {
        let num = SectionNumber::parse("01.02.03.04.05.06.07");
        assert!(num.is_none());
    }

    #[test]
    fn test_extend() {
        let base = SectionNumber::parse("01.02").unwrap();
        let extended = base.extend(&[3, 4]).unwrap();
        assert_eq!(extended.parts(), &[1, 2, 3, 4]);
        assert_eq!(extended.to_string(), "1.2.3.4");
    }

    #[test]
    fn test_extend_exceeds_max() {
        let base = SectionNumber::parse("01.02.03.04").unwrap();
        let result = base.extend(&[5, 6, 7]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SectionNumberError::DepthExceeded(7)
        ));
    }

    #[test]
    fn test_is_parent_marker() {
        let num1 = SectionNumber::parse("01.00").unwrap();
        assert!(num1.is_parent_marker());

        let num2 = SectionNumber::parse("01.02").unwrap();
        assert!(!num2.is_parent_marker());

        let num3 = SectionNumber::parse("01.02.00").unwrap();
        assert!(num3.is_parent_marker());
    }

    #[test]
    fn test_without_parent_marker() {
        let num1 = SectionNumber::parse("01.02.00").unwrap();
        let parent = num1.without_parent_marker().unwrap();
        assert_eq!(parent.parts(), &[1, 2]);

        let num2 = SectionNumber::parse("01.02").unwrap();
        assert!(num2.without_parent_marker().is_none());
    }

    #[test]
    fn test_ordering() {
        let num1 = SectionNumber::parse("01.01").unwrap();
        let num2 = SectionNumber::parse("01.02").unwrap();
        let num3 = SectionNumber::parse("02.01").unwrap();

        assert!(num1 < num2);
        assert!(num2 < num3);
        assert!(num1 < num3);
    }

    #[test]
    fn test_depth() {
        let num1 = SectionNumber::parse("01").unwrap();
        assert_eq!(num1.depth(), 0);

        let num2 = SectionNumber::parse("01.02").unwrap();
        assert_eq!(num2.depth(), 1);

        let num3 = SectionNumber::parse("01.02.03").unwrap();
        assert_eq!(num3.depth(), 2);
    }

    #[test]
    fn test_effective_heading_level() {
        // Section 01 (depth 0): effective level = 0 + 1 = 1
        let num1 = SectionNumber::parse("01").unwrap();
        assert_eq!(num1.effective_heading_level(), 1);

        // Section 01.02 (depth 1): effective level = 1 + 1 = 2
        let num2 = SectionNumber::parse("01.02").unwrap();
        assert_eq!(num2.effective_heading_level(), 2);

        // Section 03.01.01 (depth 2): effective level = 2 + 1 = 3
        let num3 = SectionNumber::parse("03.01.01").unwrap();
        assert_eq!(num3.effective_heading_level(), 3);

        // Section 3.1.2.1 (h2 under 3.1.2): depth 3, effective level = 3 + 1 = 4
        let num4 = SectionNumber::parse("03.01.02.01").unwrap();
        assert_eq!(num4.effective_heading_level(), 4);

        // Deep section: depth 3, effective level = 3 + 1 = 4
        let num5 = SectionNumber::parse("01.02.03.04").unwrap();
        assert_eq!(num5.effective_heading_level(), 4);

        // Maximum depth section: depth 5, effective level = 5 + 1 = 6
        let num6 = SectionNumber::parse("01.02.03.04.05.06").unwrap();
        assert_eq!(num6.effective_heading_level(), 6);
    }
}
