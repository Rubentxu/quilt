//! Reapply Template use case (F15).
//!
//! Re-applies template properties to an existing block with conflict detection.
//! V1 modes: `OverrideAll` (LWW) and `PreserveManual` (timestamp-based).

use crate::errors::ApplicationError;
use crate::use_cases::TemplateUseCases;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

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

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
}

// ── ReapplyTemplateUseCase trait ────────────────────────────────────────────────

/// Hidden property key for tracking when a template was last applied.
pub const TEMPLATE_APPLIED_AT_KEY: &str = "_template_applied_at";

/// Use case for reapplying a template to an existing block.
#[async_trait]
pub trait ReapplyTemplateUseCase: Send + Sync {
    /// Reapply the named template's properties to the given block.
    ///
    /// **OverrideAll mode**: overwrites ALL template properties on the block,
    /// regardless of manual edits. Sets `_template_applied_at` to now.
    ///
    /// **PreserveManual mode**: only overwrites properties that have NOT been
    /// manually edited since the last template application.
    /// A manual edit is detected when `block.updated_at > _template_applied_at`.
    /// If `_template_applied_at` is missing (T0 state), all properties are
    /// considered "manual" and preserved.
    async fn reapply(
        &self,
        template_name: &str,
        block_id: Uuid,
        mode: ReapplyMode,
    ) -> Result<ReapplyResult, ReapplyError>;
}

// ── Implementation ─────────────────────────────────────────────────────────────

/// Concrete implementation backed by TemplateUseCases + BlockRepository.
pub struct ReapplyTemplateUseCaseImpl<TC: TemplateUseCases + ?Sized, BR> {
    template_use_cases: Arc<TC>,
    block_repo: Arc<BR>,
}

impl<TC: TemplateUseCases + ?Sized, BR> ReapplyTemplateUseCaseImpl<TC, BR> {
    pub fn new(template_use_cases: Arc<TC>, block_repo: Arc<BR>) -> Self {
        Self {
            template_use_cases,
            block_repo,
        }
    }
}

#[async_trait]
impl<TC: TemplateUseCases + ?Sized + 'static, BR: quilt_domain::repositories::BlockRepository + 'static>
    ReapplyTemplateUseCase for ReapplyTemplateUseCaseImpl<TC, BR>
{
    #[instrument(skip(self))]
    async fn reapply(
        &self,
        template_name: &str,
        block_id: Uuid,
        mode: ReapplyMode,
    ) -> Result<ReapplyResult, ReapplyError> {
        // 1. Fetch the template schema
        let schema = self
            .template_use_cases
            .get_template_schema(template_name)
            .await
            .map_err(|_e| ReapplyError::TemplateNotFound(template_name.to_string()))?
            .ok_or(ReapplyError::TemplateNotFound(template_name.to_string()))?;

        // 2. Fetch the block
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| ReapplyError::Infrastructure(format!("block repo: {e}")))?
            .ok_or(ReapplyError::BlockNotFound(block_id.to_string()))?;

        // 3. Determine the template-applied-at timestamp from block properties
        let template_applied_at = block.properties.get(TEMPLATE_APPLIED_AT_KEY).and_then(|v| {
            if let quilt_domain::value_objects::PropertyValue::String(s) = v {
                DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else {
                None
            }
        });

        // 4. Build the new properties map
        let now = Utc::now();
        let mut applied = Vec::new();
        let mut preserved = Vec::new();
        let mut overwritten = Vec::new();
        let mut new_properties = block.properties.clone();

        for template_prop in &schema.properties {
            let key = &template_prop.key;

            // Skip reserved keys
            if matches!(key.as_str(), "template" | "type" | "collapsed") {
                continue;
            }

            let is_manual_edit = if let Some(applied_at) = template_applied_at {
                // If block was updated AFTER template application, it's a manual edit
                block.updated_at > applied_at
            } else {
                // No timestamp = T0 state = all properties are "manual"
                true
            };

            match mode {
                ReapplyMode::OverrideAll => {
                    // LWW: always apply the template value
                    let prop_value = quilt_domain::value_objects::PropertyValue::String(
                        template_prop.value.clone(),
                    );
                    new_properties.insert(key.clone(), prop_value);
                    applied.push(key.clone());
                    if block.properties.contains_key(key) {
                        overwritten.push(key.clone());
                    }
                }
                ReapplyMode::PreserveManual => {
                    if is_manual_edit {
                        // Keep the manual edit
                        preserved.push(key.clone());
                    } else {
                        // Apply template value
                        let prop_value = quilt_domain::value_objects::PropertyValue::String(
                            template_prop.value.clone(),
                        );
                        new_properties.insert(key.clone(), prop_value);
                        applied.push(key.clone());
                    }
                }
            }
        }

        // 5. Update _template_applied_at timestamp
        new_properties.insert(
            TEMPLATE_APPLIED_AT_KEY.to_string(),
            quilt_domain::value_objects::PropertyValue::String(now.to_rfc3339()),
        );

        // 6. Persist the updated block
        let mut updated_block = block;
        updated_block.properties = new_properties;
        updated_block.updated_at = now;
        self.block_repo
            .update(&updated_block)
            .await
            .map_err(|e| ReapplyError::Infrastructure(format!("block update: {e}")))?;

        Ok(ReapplyResult {
            applied,
            preserved,
            overwritten,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests for OverrideAll/PreserveManual with real repos
    // are in crates/quilt-application/tests/reapply_template_tests.rs

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
        let e = ReapplyError::Infrastructure("connection failed".to_string());
        assert_eq!(format!("{}", e), "Infrastructure error: connection failed");
    }
}
