//! WASM projection resolver — orchestrator for the V1 contract registry.
//!
//! Mirrors the server's `quilt_application::use_cases::projection_resolver`
//! (slice #4) but operates on `BlockDto` + `serde_json::Value` and uses
//! pure-function contracts (no `Arc<dyn Fn>` closures, which cannot
//! cross the WASM boundary).
//!
//! # Resolution algorithm
//!
//! 1. **Collect candidates**: iterate all 6 contracts, evaluate
//!    `contract.matches(&block)`. The candidate set is the subset
//!    whose contracts all matched.
//! 2. **Score candidates**: each candidate's score is
//!    `-(priority as f64)` (the WASM contracts do not support
//!    `Arc<dyn Fn>` scores — priority is the only scoring dimension).
//! 3. **Pick the winner**:
//!    - If the candidate set is empty, the winner is the
//!      Default contract.
//!    - If the candidate set has exactly one entry, that entry
//!      wins.
//!    - If the candidate set has 2+ entries, the one with the
//!      highest score wins; on a tie, the one with the smallest
//!      priority wins; on a further tie (equal score and equal
//!      priority), the resolver falls back to the Default
//!      contract and materializes a `WasmProjectionConflict` in
//!      the view's `conflicts` array.
//! 4. **Build the view**: start with the base surface
//!    (text = `block.content`, children = `block.refs`,
//!    properties = the block's properties map flattened to
//!    `BTreeMap<String, serde_json::Value>`). If the winner is a
//!    specialized contract, append its decorations and add
//!    `("projection", <winner_id>)` to the properties.
//! 5. **Always set `view.wasm_contract_id`** to the winner's id
//!    (or `"default"` for the fallback).
//! 6. **Set `view.wasm_had_conflict`** to `true` if a conflict
//!    was detected, `false` otherwise.

use crate::projection::view::{WasmDecoration, WasmProjectionConflict, WasmProjectionView};
use crate::types::BlockDto;
use std::collections::BTreeMap;

/// A V1 projection contract — pure function (no closures).
///
/// Implementors are unit structs (e.g., `TaskContract`) that compute
/// the match and decoration logic in pure-function style. The
/// `priority` is the only scoring dimension (matches the server's
/// `score = -(priority as f64)`).
pub trait WasmContract: Send + Sync {
    /// The contract id (e.g., `"task"`, `"default"`).
    fn id(&self) -> &'static str;

    /// The contract priority (smaller = more specific). Default
    /// contracts use `u32::MAX`.
    fn priority(&self) -> u32;

    /// Evaluate the contract's predicate(s) against the block.
    fn matches(&self, block: &BlockDto) -> bool;

    /// Compute the contract's contribution to the view (additions
    /// to the base surface — decorations, derived properties).
    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration>;
}

/// A registered (contract, priority) pair used by the resolver's
/// scoring algorithm.
struct RegisteredContract {
    contract: Box<dyn WasmContract>,
}

impl RegisteredContract {
    fn score(&self) -> f64 {
        -(self.contract.priority() as f64)
    }
}

/// The V1 projection registry — 6 built-in contracts.
pub struct WasmProjectionResolver {
    contracts: Vec<RegisteredContract>,
}

impl WasmProjectionResolver {
    /// Build the V1 registry. Constructs the 6 contracts in
    /// priority order. Build-time assertion: all priorities are
    /// unique (mirrors the server's `StaticProjectionRegistry::v1`).
    pub fn v1() -> Self {
        use crate::projection::contracts::{
            DateContract, DefaultContract, HeadingContract, LinkContract, MediaContract,
            TaskContract,
        };

        let contracts: Vec<RegisteredContract> = vec![
            RegisteredContract {
                contract: Box::new(TaskContract),
            },
            RegisteredContract {
                contract: Box::new(HeadingContract),
            },
            RegisteredContract {
                contract: Box::new(MediaContract),
            },
            RegisteredContract {
                contract: Box::new(DateContract),
            },
            RegisteredContract {
                contract: Box::new(LinkContract),
            },
            RegisteredContract {
                contract: Box::new(DefaultContract),
            },
        ];

        // Build-time assertion: all priorities are unique.
        let mut priorities: Vec<u32> = contracts.iter().map(|rc| rc.contract.priority()).collect();
        priorities.sort_unstable();
        let original_len = priorities.len();
        priorities.dedup();
        assert_eq!(
            priorities.len(),
            original_len,
            "Duplicate projection contract priorities in V1 registry"
        );

        Self { contracts }
    }

    /// Resolve the winning projection for a block.
    ///
    /// The algorithm is documented in the module-level doc. The
    /// resolver never panics; on any internal error it returns a
    /// default view (the `if scored.is_empty()` branch).
    pub fn resolve(&self, block: &BlockDto) -> WasmProjectionView {
        // 1. Collect candidates
        let candidates: Vec<&RegisteredContract> = self
            .contracts
            .iter()
            .filter(|rc| rc.contract.matches(block))
            .collect();

        // 2. Score candidates
        let scored: Vec<(&RegisteredContract, f64)> =
            candidates.iter().map(|rc| (*rc, rc.score())).collect();

        // 3. Pick winner
        if scored.is_empty() {
            // No contract matched (only possible if DefaultContract's
            // `matches` returns false, which it never does — but the
            // branch is defensive). Fall back to the Default contract
            // by ID lookup.
            return self.fallback_to_default(block, &[]);
        }

        // Find the highest score
        let top_score = scored
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::NEG_INFINITY, f64::max);

        // Filter to top-scoring candidates (using epsilon for f64 safety)
        const EPS: f64 = 1e-9;
        let top_candidates: Vec<&(&RegisteredContract, f64)> = scored
            .iter()
            .filter(|(_, s)| (s - top_score).abs() < EPS)
            .collect();

        if top_candidates.len() == 1 {
            // Unambiguous winner
            let (rc, _) = top_candidates[0];
            self.build_winner_view(block, rc)
        } else {
            // Tie: pick smallest priority
            let winner = top_candidates
                .iter()
                .min_by_key(|(rc, _)| rc.contract.priority())
                .expect("top_candidates is non-empty");

            if top_candidates
                .iter()
                .all(|(rc, _)| rc.contract.priority() == winner.0.contract.priority())
            {
                // Equal score AND equal priority: genuine conflict.
                // Fall back to Default and materialize the conflict.
                let candidates: Vec<String> = top_candidates
                    .iter()
                    .map(|(rc, _)| rc.contract.id().to_string())
                    .collect();
                self.fallback_to_default_with_conflict(
                    block,
                    &candidates,
                    &format!(
                        "tied score: {} contracts with score {}",
                        top_candidates.len(),
                        top_score
                    ),
                )
            } else {
                // Priority tie-breaker resolved it
                self.build_winner_view(block, winner.0)
            }
        }
    }

    /// Build a view with the winning contract's decorations.
    fn build_winner_view(&self, block: &BlockDto, rc: &RegisteredContract) -> WasmProjectionView {
        let contract_id = rc.contract.id();
        let decorations = rc.contract.apply(block);

        // Base surface
        let mut properties: BTreeMap<String, serde_json::Value> = base_properties(block);
        properties.insert("projection".to_string(), serde_json::json!(contract_id));

        WasmProjectionView {
            text: block.content.clone(),
            links: Vec::new(),
            children: block.refs.clone(),
            decorations,
            conflicts: Vec::new(),
            properties,
            wasm_source: true,
            wasm_contract_id: contract_id.to_string(),
            wasm_had_conflict: false,
        }
    }

    /// Fall back to the Default contract (no match found).
    fn fallback_to_default(
        &self,
        block: &BlockDto,
        _tied_candidates: &[&RegisteredContract],
    ) -> WasmProjectionView {
        let mut properties: BTreeMap<String, serde_json::Value> = base_properties(block);
        properties.insert("projection".to_string(), serde_json::json!("default"));

        WasmProjectionView {
            text: block.content.clone(),
            links: Vec::new(),
            children: block.refs.clone(),
            decorations: Vec::new(),
            conflicts: Vec::new(),
            properties,
            wasm_source: true,
            wasm_contract_id: "default".to_string(),
            wasm_had_conflict: false,
        }
    }

    /// Fall back to the Default contract AND materialize the conflict.
    fn fallback_to_default_with_conflict(
        &self,
        block: &BlockDto,
        candidates: &[String],
        reason: &str,
    ) -> WasmProjectionView {
        let mut view = self.fallback_to_default(block, &[]);
        view.conflicts.push(WasmProjectionConflict {
            reason: reason.to_string(),
            candidates: candidates.to_vec(),
            winner: None,
            block_id: block.id.clone(),
        });
        view.wasm_had_conflict = true;
        view
    }

    /// Number of registered contracts (always 6 for V1).
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    /// Whether the registry is empty (always false for V1).
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }
}

/// Extract a `BTreeMap` view of a block's properties for deterministic
/// iteration in the view's `properties` field.
fn base_properties(block: &BlockDto) -> BTreeMap<String, serde_json::Value> {
    block
        .properties
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}
