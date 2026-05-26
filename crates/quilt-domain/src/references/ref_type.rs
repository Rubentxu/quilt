//! RefType — the type of a reference between entities
//!
//! References are typed to distinguish between different kinds of links:
//! - `PageRef`: `[[Page Name]]` — link to a page
//! - `BlockRef`: `((block-uuid))` — link to a specific block
//! - `Tag`: `#tag` — a tag applied to content
//! - `Alias`: an alternative name for a page

use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of a reference.
///
/// Each variant corresponds to a different linking mechanism in Quilt.
/// These types are stored in the `ref_type` column in the refs table
/// with string values `'page_ref'`, `'block_ref'`, `'tag'`, `'alias'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefType {
    /// Reference to a page via `[[Page Name]]`
    PageRef,
    /// Reference to a block via `((block-uuid))`
    BlockRef,
    /// A `#tag` applied to content
    Tag,
    /// An alias (alternative name) for a page
    Alias,
}

impl RefType {
    /// Returns the database representation of this ref type.
    pub fn as_str(&self) -> &'static str {
        match self {
            RefType::PageRef => "page_ref",
            RefType::BlockRef => "block_ref",
            RefType::Tag => "tag",
            RefType::Alias => "alias",
        }
    }

    /// Parse a ref type from its database string representation.
    ///
    /// Returns `None` if the string does not match any known ref type.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "page_ref" => Some(RefType::PageRef),
            "block_ref" => Some(RefType::BlockRef),
            "tag" => Some(RefType::Tag),
            "alias" => Some(RefType::Alias),
            _ => None,
        }
    }
}

impl fmt::Display for RefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_type_roundtrip() {
        for variant in &[
            RefType::PageRef,
            RefType::BlockRef,
            RefType::Tag,
            RefType::Alias,
        ] {
            let s = variant.as_str();
            let parsed = RefType::from_str(s).unwrap();
            assert_eq!(*variant, parsed);
        }
    }

    #[test]
    fn test_ref_type_invalid() {
        assert!(RefType::from_str("invalid").is_none());
        assert!(RefType::from_str("").is_none());
    }

    #[test]
    fn test_ref_type_display() {
        assert_eq!(RefType::PageRef.to_string(), "page_ref");
        assert_eq!(RefType::BlockRef.to_string(), "block_ref");
        assert_eq!(RefType::Tag.to_string(), "tag");
        assert_eq!(RefType::Alias.to_string(), "alias");
    }

    #[test]
    fn test_ref_type_serde() {
        let json = serde_json::to_string(&RefType::PageRef).unwrap();
        assert_eq!(json, "\"page_ref\"");
        let parsed: RefType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, RefType::PageRef);
    }
}
