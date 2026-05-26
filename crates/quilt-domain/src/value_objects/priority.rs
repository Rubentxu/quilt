//! Priority value object - task priority levels

use std::fmt;

/// Priority represents the priority level of a task.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Priority {
    /// High priority (A)
    A,
    /// Medium priority (B)
    #[default]
    B,
    /// Low priority (C)
    C,
}

impl Priority {
    /// Get the display character
    pub fn as_char(&self) -> char {
        match self {
            Priority::A => 'A',
            Priority::B => 'B',
            Priority::C => 'C',
        }
    }

    /// Get the Logseq property value
    pub fn as_property_value(&self) -> &'static str {
        match self {
            Priority::A => "A",
            Priority::B => "B",
            Priority::C => "C",
        }
    }

    /// Parse from character
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'A' => Some(Priority::A),
            'B' => Some(Priority::B),
            'C' => Some(Priority::C),
            _ => None,
        }
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Option<Self> {
        Self::from_char(s.chars().next()?)
    }

    /// Get default priority
    pub fn default_priority() -> Self {
        Priority::B
    }

    /// Get all valid priorities
    pub fn all() -> &'static [Priority] {
        &[Priority::A, Priority::B, Priority::C]
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::A < Priority::B);
        assert!(Priority::B < Priority::C);
    }

    #[test]
    fn test_from_char() {
        assert_eq!(Priority::from_char('A'), Some(Priority::A));
        assert_eq!(Priority::from_char('a'), Some(Priority::A));
        assert_eq!(Priority::from_char('X'), None);
    }
}
