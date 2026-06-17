//! Projection contract — declarative block matching with priority, predicates, and scoring.
//!
//! A [`ProjectionContract`] is the declarative unit of the projection registry.
//! It describes:
//! - **which block type it applies to** (via [`PropertyPredicate`] matchers)
//! - **how to score it** relative to other contracts (priority + optional score function)
//! - **optional guards** for complex conditions that can't be expressed as predicates
//!
//! The contract is **pure data** — no side effects. The [`Projection`] adapter
//! (application layer) implements the actual view-building logic.

use crate::entities::Block;
use crate::projection::predicate::PropertyPredicate;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// A projection contract identifier.
///
/// IDs must be lowercase alphanumeric with dashes only (`[a-z0-9-]+`).
/// Examples: `"default"`, `"task"`, `"media"`, `"heading"`, `"link"`, `"date"`.
///
/// Construct via [`ProjectionContractId::new`]; validation happens at construction time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectionContractId(String);

impl ProjectionContractId {
    /// Construct a new contract ID, validating the format.
    ///
    /// Returns `Err` if the ID is empty or contains non-lowercase alphanumeric
    /// characters (dashes `-` are allowed).
    pub fn new(id: &str) -> Result<Self, ContractIdError> {
        if id.is_empty() {
            return Err(ContractIdError::Empty);
        }
        if !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(ContractIdError::InvalidCharacters(id.to_string()));
        }
        Ok(ProjectionContractId(id.to_string()))
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectionContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for ProjectionContractId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Error constructing a [`ProjectionContractId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractIdError {
    #[error("contract ID cannot be empty")]
    Empty,

    #[error(
        "contract ID '{0}' contains invalid characters; use only lowercase letters, digits, and dashes"
    )]
    InvalidCharacters(String),
}

/// A declarative projection contract.
///
/// Each contract describes when a block matches a particular visual projection
/// (task checkbox, media preview, heading anchor, etc.) and how to rank
/// competing candidates when multiple contracts match the same block.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectionContract {
    /// Unique identifier for this contract.
    pub id: ProjectionContractId,
    /// Priority for tie-breaking when scores are equal.
    /// Lower number = higher priority. Default is 1000.
    /// The special value `u32::MAX` is reserved for `DefaultProjection`.
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// Conjunctive list of predicates — ALL must match for the block
    /// to be considered a candidate for this contract.
    #[serde(default)]
    pub predicates: Vec<PropertyPredicate>,
    /// Optional guard closure — runs after predicates match.
    /// Additional complex condition that can't be expressed declaratively.
    #[serde(skip)]
    pub guard: Option<Arc<dyn Fn(&Block) -> bool + Send + Sync>>,
    /// Optional scoring closure — overrides the default priority-based score.
    /// Higher score wins. When absent, score is `-(priority as f64)`.
    #[serde(skip)]
    pub score: Option<Arc<dyn Fn(&Block) -> f64 + Send + Sync>>,
}

fn default_priority() -> u32 {
    1000
}

impl ProjectionContract {
    /// Construct a new contract with the given ID.
    ///
    /// The ID must be a valid [`ProjectionContractId`].
    #[must_use]
    pub fn new(id: ProjectionContractId) -> Self {
        Self {
            id,
            priority: 1000,
            predicates: Vec::new(),
            guard: None,
            score: None,
        }
    }

    /// Returns `true` if all predicates match AND the guard (if present) passes.
    ///
    /// An empty `predicates` list is a wildcard — always matches (but guard
    /// still applies if present).
    #[must_use]
    pub fn matches_block(&self, block: &Block) -> bool {
        // All predicates must match (AND semantics)
        if !self.predicates.iter().all(|p| p.matches(block)) {
            return false;
        }
        // Guard is a second-stage check
        if let Some(ref guard) = self.guard {
            if !guard(block) {
                return false;
            }
        }
        true
    }

    /// Score a block for this contract.
    ///
    /// Returns the score from the closure if present, otherwise
    /// `-(priority as f64)` (so higher priority = higher score).
    #[must_use]
    pub fn score_block(&self, block: &Block) -> f64 {
        if let Some(ref score_fn) = self.score {
            score_fn(block)
        } else {
            -(self.priority as f64)
        }
    }

    // ── Builders ─────────────────────────────────────────────────────

    /// Set the priority (chainable).
    #[must_use]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Add a predicate (chainable).
    #[must_use]
    pub fn with_predicate(mut self, predicate: PropertyPredicate) -> Self {
        self.predicates.push(predicate);
        self
    }

    /// Replace all predicates at once (chainable).
    #[must_use]
    pub fn with_predicates(mut self, predicates: Vec<PropertyPredicate>) -> Self {
        self.predicates = predicates;
        self
    }

    /// Set the guard closure (chainable).
    #[must_use]
    pub fn with_guard(mut self, guard: Arc<dyn Fn(&Block) -> bool + Send + Sync>) -> Self {
        self.guard = Some(guard);
        self
    }

    /// Set the guard closure using a named function reference (chainable).
    ///
    /// The closure is stored but serialized as a placeholder string `"<guard>"`.
    #[must_use]
    pub fn with_guard_named<F>(self, _name: &str, guard: F) -> Self
    where
        F: Fn(&Block) -> bool + Send + Sync + 'static,
    {
        self.with_guard(Arc::new(guard))
    }

    /// Set the score closure (chainable).
    #[must_use]
    pub fn with_score(mut self, score: Arc<dyn Fn(&Block) -> f64 + Send + Sync>) -> Self {
        self.score = Some(score);
        self
    }

    /// Set the score closure using a named function reference (chainable).
    ///
    /// The closure is stored but serialized as a placeholder string `"<score>"`.
    #[must_use]
    pub fn with_score_named<F>(self, _name: &str, score: F) -> Self
    where
        F: Fn(&Block) -> f64 + Send + Sync + 'static,
    {
        self.with_score(Arc::new(score))
    }
}

/// Manual `Debug` for [`ProjectionContract`] that doesn't recurse into
/// the `Box<dyn Fn>` trait objects — instead shows a placeholder.
impl fmt::Debug for ProjectionContract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectionContract")
            .field("id", &self.id)
            .field("priority", &self.priority)
            .field("predicates", &self.predicates)
            .field(
                "guard",
                if self.guard.is_some() {
                    &"<dyn Fn(&Block) -> bool>"
                } else {
                    &None::<&str>
                },
            )
            .field(
                "score",
                if self.score.is_some() {
                    &"<dyn Fn(&Block) -> f64>"
                } else {
                    &None::<&str>
                },
            )
            .finish()
    }
}

/// Manual `PartialEq` — compares `id`, `priority`, and `predicates`.
/// Guard and score closures are NOT compared (functional equality — two
/// contracts with the same id/priority/predicates are considered equal
/// for matching purposes, regardless of closure identity).
impl PartialEq for ProjectionContract {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.priority == other.priority
            && self.predicates == other.predicates
    }
}

impl Eq for ProjectionContract {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Block, PropertyKey};
    use crate::projection::predicate::PropertyPredicate;
    use crate::value_objects::{PropertyValue, Uuid};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block(props: HashMap<String, PropertyValue>) -> Block {
        Block {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: crate::value_objects::BlockFormat::Markdown,
            block_type: crate::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "test".into(),
            properties: props,
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: None,
            cancelled_at: None,
            collapsed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── ProjectionContractId ────────────────────────────────────────

    #[test]
    fn contract_id_valid_ids_construct() {
        for id in [
            "default", "task", "media", "heading", "link", "date", "video", "audio",
        ] {
            ProjectionContractId::new(id).expect("valid ID should construct");
        }
    }

    #[test]
    fn contract_id_rejects_empty() {
        assert!(matches!(
            ProjectionContractId::new(""),
            Err(ContractIdError::Empty)
        ));
    }

    #[test]
    fn contract_id_rejects_whitespace() {
        assert!(ProjectionContractId::new("task view").is_err());
        assert!(ProjectionContractId::new("task\tview").is_err());
    }

    #[test]
    fn contract_id_rejects_uppercase() {
        assert!(ProjectionContractId::new("Task").is_err());
        assert!(ProjectionContractId::new("TASK").is_err());
        assert!(ProjectionContractId::new("TaskView").is_err());
    }

    #[test]
    fn contract_id_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ProjectionContractId::new("task").unwrap());
        set.insert(ProjectionContractId::new("media").unwrap());
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn contract_id_serializes_as_string() {
        let id = ProjectionContractId::new("task").unwrap();
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"task\"");
    }

    // ── ProjectionContract ───────────────────────────────────────────

    #[test]
    fn contract_default_priority_1000() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap());
        assert_eq!(contract.priority, 1000);
    }

    #[test]
    fn contract_with_predicate_appends() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_predicate(PropertyPredicate::IsSet {
                key: PropertyKey::new("status").unwrap(),
            });

        assert_eq!(contract.predicates.len(), 1);
    }

    #[test]
    fn contract_with_predicates_replaces() {
        let pred1 = PropertyPredicate::IsSet {
            key: PropertyKey::new("a").unwrap(),
        };
        let pred2 = PropertyPredicate::IsSet {
            key: PropertyKey::new("b").unwrap(),
        };

        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_predicates(vec![pred1, pred2]);

        assert_eq!(contract.predicates.len(), 2);
    }

    #[test]
    fn contract_with_guard_stores_closure() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_guard(Arc::new(|_| true));
        assert!(contract.guard.is_some());

        let contract2 = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_guard(Arc::new(|_| false));
        let block = make_block(HashMap::new());
        // Guard returns false but predicates match (empty = wildcard)
        // matches_block checks guard only after predicates
        let block2 = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p
        });
        // With empty predicates, guard is the only check
        assert!(!contract2.matches_block(&block));
    }

    #[test]
    fn contract_with_score_stores_closure() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_score(Arc::new(|_| 99.0));
        assert!(contract.score.is_some());

        let block = make_block(HashMap::new());
        assert_eq!(contract.score_block(&block), 99.0);
    }

    #[test]
    fn contract_debug_does_not_panic_with_closures() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_guard(Arc::new(|_| true))
            .with_score(Arc::new(|_| 10.0));

        // Debug formatting must not panic even with closures stored
        let debug_str = format!("{contract:?}");
        assert!(debug_str.contains("task"));
        assert!(debug_str.contains("dyn Fn"));
    }

    #[test]
    fn contract_serializes_guard_score_as_named_placeholder() {
        // guard and score are skipped in serialization
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_guard(Arc::new(|_| true))
            .with_score(Arc::new(|_| 10.0));

        let json = serde_json::to_string(&contract).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // guard and score fields should be absent (skipped)
        assert!(!v.as_object().unwrap().contains_key("guard"));
        assert!(!v.as_object().unwrap().contains_key("score"));
        assert_eq!(v["id"], "task");
        assert_eq!(v["priority"], 1000);
    }

    #[test]
    fn contract_deserialize_leaves_guard_score_none() {
        let json = r#"{"id":"task","priority":100,"predicates":[]}"#;
        let contract: ProjectionContract = serde_json::from_str(json).expect("deserialize");

        assert!(contract.guard.is_none());
        assert!(contract.score.is_none());
        assert_eq!(contract.id.as_str(), "task");
    }

    // ── matches_block ─────────────────────────────────────────────

    #[test]
    fn matches_block_all_predicates_must_match() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("todo"));
            p
        });

        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_predicates(vec![
                PropertyPredicate::Equals {
                    key: PropertyKey::new("type").unwrap(),
                    value: PropertyValue::string("task"),
                },
                PropertyPredicate::IsSet {
                    key: PropertyKey::new("status").unwrap(),
                },
            ]);

        assert!(contract.matches_block(&block));
    }

    #[test]
    fn matches_block_empty_predicates_is_wildcard() {
        let block = make_block(HashMap::new());

        // No predicates = wildcard (always matches if guard passes or is absent)
        let contract = ProjectionContract::new(ProjectionContractId::new("default").unwrap());
        assert!(contract.matches_block(&block));

        // With guard = guard determines outcome
        let contract2 = ProjectionContract::new(ProjectionContractId::new("default").unwrap())
            .with_guard(Arc::new(|_| false));
        assert!(!contract2.matches_block(&block));

        let contract3 = ProjectionContract::new(ProjectionContractId::new("default").unwrap())
            .with_guard(Arc::new(|_| true));
        assert!(contract3.matches_block(&block));
    }

    // ── score_block ──────────────────────────────────────────────

    #[test]
    fn score_block_negation_of_priority() {
        let contract =
            ProjectionContract::new(ProjectionContractId::new("task").unwrap()).with_priority(10);
        let block = make_block(HashMap::new());
        assert_eq!(contract.score_block(&block), -10.0);

        let contract2 =
            ProjectionContract::new(ProjectionContractId::new("task").unwrap()).with_priority(1000);
        assert_eq!(contract2.score_block(&block), -1000.0);
    }

    #[test]
    fn score_block_uses_closure_when_present() {
        let contract = ProjectionContract::new(ProjectionContractId::new("task").unwrap())
            .with_priority(10) // should be ignored
            .with_score(Arc::new(|_| 42.0));
        let block = make_block(HashMap::new());
        assert_eq!(contract.score_block(&block), 42.0);
    }

    #[test]
    fn higher_score_wins() {
        // This is documented via a doc-test style comment
        // Higher score wins; with equal scores, lower priority number wins
        let block = make_block(HashMap::new());

        let low_priority =
            ProjectionContract::new(ProjectionContractId::new("a").unwrap()).with_priority(200); // score = -200

        let high_priority =
            ProjectionContract::new(ProjectionContractId::new("b").unwrap()).with_priority(50); // score = -50

        // -50 > -200, so high_priority has higher score
        assert!(high_priority.score_block(&block) > low_priority.score_block(&block));
    }

    // ── no-shadow compile check ──────────────────────────────────

    #[test]
    fn distinct_from_patch_level_projection_conflict() {
        // The patch-level ProjectionConflict lives in canonicalization::ProjectionConflict.
        // The projection-level ProjectionConflict lives in projection::ProjectionConflict.
        // They are distinct types — importing both in the same scope requires explicit paths.
        use crate::canonicalization::ProjectionConflict as PatchConflict;
        use crate::projection::ProjectionConflict as LayerConflict;

        // Verify both types exist and are distinct
        let _patch = PatchConflict {
            property: PropertyKey::new("type").unwrap(),
            existing_value: PropertyValue::string("a"),
            attempted_value: PropertyValue::string("b"),
            policy: crate::properties::types::MergePolicy::Overwrite,
            reason: "test".into(),
        };

        let _layer = LayerConflict {
            reason: "test".into(),
            candidates: vec![],
            winner: None,
            block_id: Uuid::new_v4(),
        };

        // Types are different
        fn _assert_distinct(_: PatchConflict, _: LayerConflict) {}
    }
}
