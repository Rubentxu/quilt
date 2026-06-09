//! Apply a template with its contract enforced (Q030 ROADMAP).
//!
//! Diffed out of `reapply.rs`:
//! - The old `reapply.rs` blindly copied template properties onto a
//!   block, with a single "preserve manual edits" heuristic.
//! - The new `ApplyTemplateWithContractUseCase` first validates the
//!   *contract*: every required property is present, every locked
//!   property matches the template's canonical value, and the
//!   contract version matches what the caller claims.
//!
//! This is the bridge between the **declarative** contract (in
//! `quilt-domain::entities::TemplateContract`) and the **operational**
//! mutation of a block in storage. The MCP tools call this through
//! the `ApplyTemplateWithContractUseCase` trait so the contract is
//! enforced regardless of which agent invokes the tool.

use crate::use_cases::TemplateUseCases;
use async_trait::async_trait;
use quilt_domain::entities::TemplateContract;
use quilt_domain::entities::Version;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::{PropertyValue, Uuid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

/// Result of a successful `apply_template_with_contract` call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyTemplateWithContractResult {
    /// Property keys that were applied to the block.
    pub applied: Vec<String>,
    /// Property keys that were preserved (already on the block with
    /// a non-template value the contract allowed to remain).
    pub preserved: Vec<String>,
    /// Property keys the contract rejected (locked mutation, missing,
    /// etc.). On a successful result this list is empty.
    pub rejected: Vec<String>,
    /// The contract version that was enforced.
    pub contract_version: Version,
}

impl Default for ApplyTemplateWithContractResult {
    fn default() -> Self {
        Self {
            applied: Vec::new(),
            preserved: Vec::new(),
            rejected: Vec::new(),
            contract_version: Version::new(),
        }
    }
}

/// Errors specific to the apply-with-contract use case.
#[derive(Debug, Error, PartialEq)]
pub enum ApplyTemplateWithContractError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Missing required property: {0}")]
    MissingRequiredProperty(String),

    #[error(
        "Locked property '{property}' was changed from template value '{template_value}' to '{proposed_value}'"
    )]
    LockedPropertyChanged {
        property: String,
        template_value: String,
        proposed_value: String,
    },

    #[error("Locked property '{0}' was added but not present in template")]
    LockedPropertyAdded(String),

    #[error("Version mismatch: contract is v{actual}, caller said v{expected}")]
    VersionMismatch { expected: u32, actual: u32 },

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
}

/// Use case for applying a template to a block, enforcing the
/// contract.
#[async_trait]
pub trait ApplyTemplateWithContractUseCase: Send + Sync {
    /// Apply the named template's contract to the given block.
    ///
    /// `proposed` is the set of property values the caller (agent)
    /// wants to apply. `caller_version` is the version of the
    /// contract the caller last saw. When `None`, the use case
    /// skips the version check (useful for backward-compatible
    /// flows that didn't fetch the contract first).
    async fn apply(
        &self,
        block_id: Uuid,
        template_name: &str,
        contract: &TemplateContract,
        proposed: &HashMap<String, String>,
        caller_version: Option<Version>,
    ) -> Result<ApplyTemplateWithContractResult, ApplyTemplateWithContractError>;
}

// ── Implementation ─────────────────────────────────────────────────

/// Concrete implementation.
pub struct ApplyTemplateWithContractUseCaseImpl<TC: TemplateUseCases + ?Sized, BR> {
    template_use_cases: Arc<TC>,
    block_repo: Arc<BR>,
}

impl<TC: TemplateUseCases + ?Sized, BR> ApplyTemplateWithContractUseCaseImpl<TC, BR> {
    pub fn new(template_use_cases: Arc<TC>, block_repo: Arc<BR>) -> Self {
        Self {
            template_use_cases,
            block_repo,
        }
    }
}

#[async_trait]
impl<TC, BR> ApplyTemplateWithContractUseCase for ApplyTemplateWithContractUseCaseImpl<TC, BR>
where
    TC: TemplateUseCases + ?Sized + 'static,
    BR: BlockRepository + 'static,
{
    #[instrument(skip(self, contract, proposed))]
    async fn apply(
        &self,
        block_id: Uuid,
        template_name: &str,
        contract: &TemplateContract,
        proposed: &HashMap<String, String>,
        caller_version: Option<Version>,
    ) -> Result<ApplyTemplateWithContractResult, ApplyTemplateWithContractError> {
        // 1. Fetch the template schema (so we know each property's
        //    canonical value for the locked-property check).
        let schema = self
            .template_use_cases
            .get_template_schema(template_name)
            .await
            .map_err(|e| {
                ApplyTemplateWithContractError::Infrastructure(format!("schema fetch: {e}"))
            })?
            .ok_or_else(|| {
                ApplyTemplateWithContractError::TemplateNotFound(template_name.to_string())
            })?;

        // 2. Version check.
        if let Some(v) = caller_version
            && v != *contract.version()
        {
            return Err(ApplyTemplateWithContractError::VersionMismatch {
                expected: v.as_u32(),
                actual: contract.version().as_u32(),
            });
        }

        // 3. Required-property check.
        for required in contract.required_properties() {
            if !proposed.contains_key(required.as_str()) {
                return Err(ApplyTemplateWithContractError::MissingRequiredProperty(
                    required.to_string(),
                ));
            }
        }

        // 4. Build a "template values" map from the schema for the
        //    locked-property check. The schema gives us the
        //    template's canonical value for each property, but
        //    `collect_properties` skips reserved keys (`template`,
        //    `type`, `collapsed`). For the locked-property check
        //    we need to fall back to scanning the template's raw
        //    blocks for those reserved keys.
        let mut template_values: HashMap<String, String> = schema
            .properties
            .iter()
            .map(|p| (p.key.clone(), p.value.clone()))
            .collect();
        for block in &schema.blocks {
            for (k, v) in &block.properties {
                if matches!(k.as_str(), "template" | "type" | "collapsed") {
                    template_values
                        .entry(k.clone())
                        .or_insert_with(|| v.as_display_string());
                }
            }
        }

        // 5. Locked-property check: every locked key in `proposed`
        //    must match the template's value.
        for locked in contract.locked_properties() {
            match (
                proposed.get(locked.as_str()),
                template_values.get(locked.as_str()),
            ) {
                (Some(proposed_val), Some(template_val)) => {
                    if proposed_val != template_val {
                        return Err(ApplyTemplateWithContractError::LockedPropertyChanged {
                            property: locked.to_string(),
                            template_value: template_val.clone(),
                            proposed_value: proposed_val.clone(),
                        });
                    }
                }
                (Some(_), None) => {
                    return Err(ApplyTemplateWithContractError::LockedPropertyAdded(
                        locked.to_string(),
                    ));
                }
                (None, _) => {
                    return Err(ApplyTemplateWithContractError::MissingRequiredProperty(
                        locked.to_string(),
                    ));
                }
            }
        }

        // 6. Fetch the block.
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| {
                ApplyTemplateWithContractError::Infrastructure(format!("block fetch: {e}"))
            })?
            .ok_or_else(|| ApplyTemplateWithContractError::BlockNotFound(block_id.to_string()))?;

        // 7. Build the new property map: preserve user properties
        //    that aren't part of the contract, then merge in the
        //    contract's values.
        let mut new_properties = block.properties.clone();
        let mut applied: Vec<String> = Vec::new();
        let mut preserved: Vec<String> = Vec::new();

        for (key, value) in proposed {
            if let Some(existing) = block.properties.get(key) {
                if existing.as_display_string() == *value {
                    preserved.push(key.clone());
                } else {
                    applied.push(key.clone());
                }
            } else {
                applied.push(key.clone());
            }
            new_properties.insert(key.clone(), PropertyValue::String(value.clone()));
        }

        // 8. Persist.
        let mut updated_block = block;
        updated_block.properties = new_properties;
        updated_block.updated_at = chrono::Utc::now();
        self.block_repo.update(&updated_block).await.map_err(|e| {
            ApplyTemplateWithContractError::Infrastructure(format!("block update: {e}"))
        })?;

        Ok(ApplyTemplateWithContractResult {
            applied,
            preserved,
            rejected: Vec::new(),
            contract_version: *contract.version(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_default_is_empty() {
        let r = ApplyTemplateWithContractResult::default();
        assert!(r.applied.is_empty());
        assert!(r.preserved.is_empty());
        assert!(r.rejected.is_empty());
        assert_eq!(r.contract_version.as_u32(), 1);
    }

    #[test]
    fn result_serde_roundtrip() {
        let r = ApplyTemplateWithContractResult {
            applied: vec!["title".to_string()],
            preserved: vec!["status".to_string()],
            rejected: vec![],
            contract_version: Version::new(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let parsed: ApplyTemplateWithContractResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, r);
    }

    #[test]
    fn error_display_messages() {
        let e = ApplyTemplateWithContractError::TemplateNotFound("ref".to_string());
        assert_eq!(format!("{}", e), "Template not found: ref");

        let e = ApplyTemplateWithContractError::MissingRequiredProperty("title".to_string());
        assert_eq!(format!("{}", e), "Missing required property: title");

        let e = ApplyTemplateWithContractError::VersionMismatch {
            expected: 2,
            actual: 1,
        };
        assert_eq!(
            format!("{}", e),
            "Version mismatch: contract is v1, caller said v2"
        );
    }
}
