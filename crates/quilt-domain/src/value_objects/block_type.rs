//! BlockType value object — visual/semantic kind of a block.
//!
//! Mirrors the TypeScript `BlockType` union in
//! `quilt-ui/src/shared/types/api.ts`. The 11 variants cover the
//! slash-command set: paragraph, heading1/2/3, bullet, numbered, todo,
//! quote, code, divider, image. The default for newly-created blocks
//! is [`BlockType::Paragraph`].
//!
//! ## Wire format
//!
//! The serde representation is the lowercase form of the variant name
//! (e.g. `Heading1` → `"heading1"`), matching the frontend
//! `BlockType` union exactly. SQLite stores the same lowercase string
//! in the `blocks.block_type` column.

use std::fmt;
use std::str::FromStr;

/// Visual / semantic kind of a block.
///
/// Used by the outliner to decide how to render a block (heading
/// levels, list bullets, code fences, etc.) and by slash commands to
/// change a block's kind at runtime.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    /// Plain paragraph — the default kind for new blocks.
    #[default]
    Paragraph,
    /// Top-level heading (H1).
    Heading1,
    /// Second-level heading (H2).
    Heading2,
    /// Third-level heading (H3).
    Heading3,
    /// Bulleted list item.
    Bullet,
    /// Numbered (ordered) list item.
    Numbered,
    /// Task / todo item (distinct from a plain `todo` marker).
    Todo,
    /// Block quote.
    Quote,
    /// Fenced code block.
    Code,
    /// Horizontal divider / separator.
    Divider,
    /// Image block.
    Image,
}

impl BlockType {
    /// The canonical lowercase string form of this block type.
    ///
    /// This is the value stored in the `block_type` column of the
    /// `blocks` table and the value sent to / received from the
    /// frontend over JSON. It MUST match the TypeScript `BlockType`
    /// union exactly — see `quilt-ui/src/shared/types/api.ts`.
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Paragraph => "paragraph",
            BlockType::Heading1 => "heading1",
            BlockType::Heading2 => "heading2",
            BlockType::Heading3 => "heading3",
            BlockType::Bullet => "bullet",
            BlockType::Numbered => "numbered",
            BlockType::Todo => "todo",
            BlockType::Quote => "quote",
            BlockType::Code => "code",
            BlockType::Divider => "divider",
            BlockType::Image => "image",
        }
    }

    /// Parse a block type from its lowercase string form.
    ///
    /// Returns `None` for unknown values so the caller can decide
    /// whether to fall back to a default or surface an error. We
    /// intentionally do NOT silently coerce unknown strings to
    /// [`BlockType::Paragraph`] — that's a domain decision callers
    /// should make explicitly.
    pub fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Return every variant in declaration order. Useful for slash
    /// command registries, table-column filters, and exhaustive
    /// round-trip tests.
    pub fn all() -> &'static [BlockType] {
        &[
            BlockType::Paragraph,
            BlockType::Heading1,
            BlockType::Heading2,
            BlockType::Heading3,
            BlockType::Bullet,
            BlockType::Numbered,
            BlockType::Todo,
            BlockType::Quote,
            BlockType::Code,
            BlockType::Divider,
            BlockType::Image,
        ]
    }
}

impl FromStr for BlockType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "paragraph" => Ok(BlockType::Paragraph),
            "heading1" => Ok(BlockType::Heading1),
            "heading2" => Ok(BlockType::Heading2),
            "heading3" => Ok(BlockType::Heading3),
            "bullet" => Ok(BlockType::Bullet),
            "numbered" => Ok(BlockType::Numbered),
            "todo" => Ok(BlockType::Todo),
            "quote" => Ok(BlockType::Quote),
            "code" => Ok(BlockType::Code),
            "divider" => Ok(BlockType::Divider),
            "image" => Ok(BlockType::Image),
            _ => Err(()),
        }
    }
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default must be `Paragraph` so that blocks created before
    /// the column existed (or with no explicit type) render correctly.
    #[test]
    fn test_default_is_paragraph() {
        assert_eq!(BlockType::default(), BlockType::Paragraph);
    }

    /// `parse_str` must accept the exact lowercase strings used by
    /// the frontend. This is the contract that wires Rust to TS.
    #[test]
    fn test_parse_str_accepts_canonical_strings() {
        for variant in BlockType::all() {
            let s = variant.as_str();
            let parsed =
                BlockType::parse_str(s).unwrap_or_else(|| panic!("parse_str failed for {:?}", s));
            assert_eq!(
                parsed, *variant,
                "round-trip mismatch for variant {:?} (str = {:?})",
                variant, s
            );
        }
    }

    /// The frontend ONLY sends lowercase strings. If we ever relax
    /// that, the existing variant set would silently mis-render.
    /// We document the strictness here: unknown strings MUST be
    /// rejected.
    #[test]
    fn test_parse_str_rejects_unknown_values() {
        assert_eq!(BlockType::parse_str(""), None);
        assert_eq!(BlockType::parse_str("Heading1"), None);
        assert_eq!(BlockType::parse_str("HEADING1"), None);
        assert_eq!(BlockType::parse_str("paragraph "), None);
        assert_eq!(BlockType::parse_str("p"), None);
        assert_eq!(BlockType::parse_str("unknown_kind"), None);
    }

    /// `as_str` output must match the TypeScript `BlockType` union
    /// one-to-one. This is the binding contract — if any string
    /// changes, the frontend will silently break.
    #[test]
    fn test_as_str_matches_typescript_union() {
        let expected = [
            "paragraph",
            "heading1",
            "heading2",
            "heading3",
            "bullet",
            "numbered",
            "todo",
            "quote",
            "code",
            "divider",
            "image",
        ];
        let actual: Vec<&'static str> = BlockType::all().iter().map(|v| v.as_str()).collect();
        assert_eq!(actual, expected);
    }

    /// JSON serialization must produce the lowercase string, e.g.
    /// `serde_json::to_string(&BlockType::Heading1)` is `"heading1"`.
    /// The frontend relies on this when sending PATCH bodies.
    #[test]
    fn test_serde_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&BlockType::Paragraph).unwrap(),
            "\"paragraph\""
        );
        assert_eq!(
            serde_json::to_string(&BlockType::Heading1).unwrap(),
            "\"heading1\""
        );
        assert_eq!(serde_json::to_string(&BlockType::Code).unwrap(), "\"code\"");
    }

    /// JSON deserialization must accept the lowercase string. The
    /// server-side `UpdateBlockRequest` deserializes `blockType`
    /// from this exact form.
    #[test]
    fn test_serde_deserializes_lowercase() {
        let bt: BlockType = serde_json::from_str("\"heading1\"").unwrap();
        assert_eq!(bt, BlockType::Heading1);

        let bt: BlockType = serde_json::from_str("\"paragraph\"").unwrap();
        assert_eq!(bt, BlockType::Paragraph);
    }

    /// Round-trip: serialize → deserialize → equal. This guards
    /// against serde attribute mistakes.
    #[test]
    fn test_serde_round_trip() {
        for variant in BlockType::all() {
            let json = serde_json::to_string(variant).unwrap();
            let back: BlockType = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, back);
        }
    }

    /// `Display` should match `as_str` — this lets `format!("{}")`
    /// produce the canonical wire form.
    #[test]
    fn test_display_matches_as_str() {
        for variant in BlockType::all() {
            assert_eq!(format!("{}", variant), variant.as_str());
        }
    }

    /// The number of variants is part of the public contract. If we
    /// ever add a new one, the TS union must be updated to match
    /// (or vice-versa). This test catches accidental drift.
    #[test]
    fn test_variant_count_matches_typescript_union() {
        assert_eq!(BlockType::all().len(), 11);
    }
}
