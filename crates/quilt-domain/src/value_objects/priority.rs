//! Priority value object - task priority levels

use crate::errors::DomainError;
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

    /// Get the Quilt property value
    pub fn as_property_value(&self) -> &'static str {
        match self {
            Priority::A => "A",
            Priority::B => "B",
            Priority::C => "C",
        }
    }

    /// Parse from character
    pub fn from_char(c: char) -> Result<Self, DomainError> {
        match c.to_ascii_uppercase() {
            'A' => Ok(Priority::A),
            'B' => Ok(Priority::B),
            'C' => Ok(Priority::C),
            _ => Err(DomainError::ParseError(format!(
                "Invalid priority value: {}",
                c
            ))),
        }
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Result<Self, DomainError> {
        let c = s
            .chars()
            .next()
            .ok_or_else(|| DomainError::ParseError("Empty priority string".to_string()))?;
        Self::from_char(c)
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

    // ── Ordering ──────────────────────────────────────────────────

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::A < Priority::B);
        assert!(Priority::B < Priority::C);
        assert!(Priority::A < Priority::C);
    }

    #[test]
    fn test_default_is_b() {
        assert_eq!(Priority::default(), Priority::B);
    }

    // ── from_char ─────────────────────────────────────────────────

    #[test]
    fn test_from_char_valid() {
        assert_eq!(Priority::from_char('A'), Ok(Priority::A));
        assert_eq!(Priority::from_char('a'), Ok(Priority::A));
        assert_eq!(Priority::from_char('B'), Ok(Priority::B));
        assert_eq!(Priority::from_char('b'), Ok(Priority::B));
        assert_eq!(Priority::from_char('C'), Ok(Priority::C));
        assert_eq!(Priority::from_char('c'), Ok(Priority::C));
    }

    #[test]
    fn test_from_char_invalid() {
        assert!(Priority::from_char('X').is_err());
        assert!(Priority::from_char('1').is_err());
        assert!(Priority::from_char(' ').is_err());
        assert!(Priority::from_char('\0').is_err());
    }

    // ── parse_str ─────────────────────────────────────────────────

    #[test]
    fn test_parse_str_valid() {
        assert_eq!(Priority::parse_str("A"), Ok(Priority::A));
        assert_eq!(Priority::parse_str("B"), Ok(Priority::B));
        assert_eq!(Priority::parse_str("C priority"), Ok(Priority::C));
    }

    #[test]
    fn test_parse_str_empty() {
        assert!(Priority::parse_str("").is_err());
    }

    #[test]
    fn test_parse_str_invalid() {
        assert!(Priority::parse_str("X").is_err());
        assert!(Priority::parse_str("1").is_err());
    }

    // ── as_char / as_property_value ────────────────────────────────

    #[test]
    fn test_as_char() {
        assert_eq!(Priority::A.as_char(), 'A');
        assert_eq!(Priority::B.as_char(), 'B');
        assert_eq!(Priority::C.as_char(), 'C');
    }

    #[test]
    fn test_as_property_value() {
        assert_eq!(Priority::A.as_property_value(), "A");
        assert_eq!(Priority::B.as_property_value(), "B");
        assert_eq!(Priority::C.as_property_value(), "C");
    }

    // ── Display ──────────────────────────────────────────────────

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Priority::A), "A");
        assert_eq!(format!("{}", Priority::B), "B");
        assert_eq!(format!("{}", Priority::C), "C");
    }

    // ── all() / default_priority() ────────────────────────────────

    #[test]
    fn test_all_returns_three_variants() {
        let all = Priority::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Priority::A));
        assert!(all.contains(&Priority::B));
        assert!(all.contains(&Priority::C));
    }

    #[test]
    fn test_default_priority_is_b() {
        assert_eq!(Priority::default_priority(), Priority::B);
    }

    // ── Serde roundtrip ───────────────────────────────────────────

    #[test]
    fn test_serde_roundtrip_all_variants() {
        for p in Priority::all() {
            let json = serde_json::to_string(p).unwrap();
            let restored: Priority = serde_json::from_str(&json).unwrap();
            assert_eq!(*p, restored, "roundtrip failed for {:?}", p);
        }
    }

    #[test]
    fn test_serde_rejects_unknown_variant() {
        assert!(serde_json::from_str::<Priority>("\"D\"").is_err());
        assert!(serde_json::from_str::<Priority>("\"Z\"").is_err());
    }

    #[test]
    fn test_serde_rejects_invalid_json() {
        assert!(serde_json::from_str::<Priority>("1").is_err());
        assert!(serde_json::from_str::<Priority>("null").is_err());
        assert!(serde_json::from_str::<Priority>("[]").is_err());
    }

    // ── Property-based: from_char roundtrip ───────────────────────

    #[test]
    fn proptest_from_char_roundtrip() {
        use proptest::prelude::*;
        proptest!(|(c in "[ABCabc]")| {
            // proptest generates String from regex patterns
            let ch = c.chars().next().unwrap();
            let parsed = Priority::from_char(ch);
            assert!(parsed.is_ok(), "failed to parse '{}'", ch);
            let roundtripped = parsed.unwrap().as_char().to_ascii_uppercase();
            assert_eq!(ch.to_ascii_uppercase(), roundtripped);
        });
    }
}
