//! Reapply Template use case (F15).
//!
//! Re-applies template properties to an existing block with conflict detection.
//! V1 modes: `OverrideAll` (LWW) and `PreserveManual` (timestamp-based).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Reapplication mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReapplyMode {
    /// Last-write-wins: overwrite all template properties unconditionally.
    OverrideAll,
    /// Preserve manual edits: only overwrite properties that haven't been
    /// edited since the last template application.
    PreserveManual,
}

impl Default for ReapplyMode {
    fn default() -> Self {
        ReapplyMode::PreserveManual
    }
}

/// Result of a template reapplication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReapplyResult {
    /// Keys that were applied from the template.
    pub applied: Vec<String>,
    /// Keys that existed on the block but were preserved (manual edits).
    pub preserved: Vec<String>,
    /// Keys that were overwritten (template took precedence).
    pub overwritten: Vec<String>,
}

impl Default for ReapplyResult {
    fn default() -> Self {
        Self::empty()
    }
}

impl ReapplyResult {
    /// Empty result (no changes).
    pub fn empty() -> Self {
        Self {
            applied: Vec::new(),
            preserved: Vec::new(),
            overwritten: Vec::new(),
        }
    }
}

/// Errors specific to the reapply use case.
#[derive(Debug, Error, PartialEq)]
pub enum ReapplyError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Invalid mode: {0}")]
    InvalidMode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reapply_mode_serde_override_all() {
        let json = r#""override_all""#;
        let mode: ReapplyMode = serde_json::from_str(json).expect("should parse");
        assert_eq!(mode, ReapplyMode::OverrideAll);
        let back = serde_json::to_string(&mode).expect("should serialize");
        assert_eq!(back, "\"override_all\"");
    }

    #[test]
    fn reapply_mode_serde_preserve_manual() {
        let json = r#""preserve_manual""#;
        let mode: ReapplyMode = serde_json::from_str(json).expect("should parse");
        assert_eq!(mode, ReapplyMode::PreserveManual);
        let back = serde_json::to_string(&mode).expect("should serialize");
        assert_eq!(back, "\"preserve_manual\"");
    }

    #[test]
    fn reapply_mode_default_is_preserve_manual() {
        let mode = ReapplyMode::default();
        assert_eq!(mode, ReapplyMode::PreserveManual);
    }

    #[test]
    fn reapply_mode_unknown_value() {
        let json = r#""unknown_mode""#;
        let result: Result<ReapplyMode, _> = serde_json::from_str(json);
        result.expect_err("unknown mode should fail to deserialize");
    }

    #[test]
    fn reapply_result_empty() {
        let r = ReapplyResult::empty();
        assert!(r.applied.is_empty());
        assert!(r.preserved.is_empty());
        assert!(r.overwritten.is_empty());
    }

    #[test]
    fn reapply_result_serde() {
        let r = ReapplyResult {
            applied: vec!["status".to_string()],
            preserved: vec!["rating".to_string()],
            overwritten: vec!["priority".to_string()],
        };
        let json = serde_json::to_string(&r).expect("should serialize");
        let parsed: ReapplyResult = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed, r);
    }

    #[test]
    fn reapply_error_display() {
        let e = ReapplyError::TemplateNotFound("my-template".to_string());
        assert_eq!(format!("{}", e), "Template not found: my-template");
        let e = ReapplyError::BlockNotFound("block-123".to_string());
        assert_eq!(format!("{}", e), "Block not found: block-123");
        let e = ReapplyError::InvalidMode("bad".to_string());
        assert_eq!(format!("{}", e), "Invalid mode: bad");
    }
}
