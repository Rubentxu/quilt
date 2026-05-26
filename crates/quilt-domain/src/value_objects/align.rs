//! Align value object - alignment for assets

use std::fmt;

/// Align represents the alignment of an embedded asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

impl Align {
    /// Get the CSS text-align value
    pub fn css_value(&self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        }
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "left" | "l" => Some(Align::Left),
            "center" | "c" | "centre" => Some(Align::Center),
            "right" | "r" => Some(Align::Right),
            _ => None,
        }
    }
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Align::Left => write!(f, "left"),
            Align::Center => write!(f, "center"),
            Align::Right => write!(f, "right"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_value() {
        assert_eq!(Align::Left.css_value(), "left");
        assert_eq!(Align::Center.css_value(), "center");
        assert_eq!(Align::Right.css_value(), "right");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Align::parse_str("left"), Some(Align::Left));
        assert_eq!(Align::parse_str("center"), Some(Align::Center));
        assert_eq!(Align::parse_str("right"), Some(Align::Right));
        assert_eq!(Align::parse_str("center"), Some(Align::Center));
    }
}
