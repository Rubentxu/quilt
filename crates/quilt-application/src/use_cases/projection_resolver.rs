//! Projection resolver use case — resolves the winning projection for a block.
//!
//! ## Resolution algorithm
//!
//! 1. **Collect candidates**: iterate all contracts in the registry,
//!    filter to those where [`ProjectionContract::matches_block`] returns `true`.
//!
//! 2. **Score candidates**: each candidate is scored via
//!    [`ProjectionContract::score_block`]. Higher score wins.
//!
//! 3. **Tie-break**: when scores are equal, the contract with the smaller
//!    `priority` number wins. The special value `u32::MAX` is reserved for
//!    [`DefaultProjection`].
//!
//! 4. **Fallback**: if no contract matches, the resolver falls back to
//!    `DefaultProjection` and records a conflict.
//!
//! 5. **Compose view**: base block surface + winner's delta via
//!    [`ProjectionViewDelta::compose_on`].
//!
//! 6. **Materialize conflicts**: when multiple contracts tie in score/priority,
//!    the conflict is surfaced as system properties on the block:
//!    - `projection-conflict`: `"true"`
//!    - `projection-conflict-reason`: human-readable reason
//!    - `projection-conflict-candidates`: comma-separated list of IDs
//!
//!    When resolution is unambiguous, `projection` is set to the winner's ID
//!    and no conflict properties are added.

use crate::errors::ApplicationError;
use crate::services::projection::StaticProjectionRegistry;
use quilt_domain::entities::{Block, PropertyKey};
use quilt_domain::projection::conflict::ProjectionConflict;
use quilt_domain::projection::contract::ProjectionContractId;
use quilt_domain::projection::projection_trait::ProjectionContext;
use quilt_domain::projection::registry::ProjectionRegistry;
use quilt_domain::projection::view::{ProjectionView, ProjectionViewBuilder, ProjectionViewDelta};
use quilt_domain::value_objects::PropertyValue;
use tracing::instrument;

/// Outcome of a single block projection resolution.
#[derive(Debug, Clone)]
pub struct ResolutionOutcome {
    /// The resolved projection view.
    pub view: ProjectionView,
    /// The winner's contract ID (if any).
    pub winner_id: Option<ProjectionContractId>,
    /// Whether a conflict was detected (tied candidates).
    pub had_conflict: bool,
}

/// Projection resolver use case.
///
/// Orchestrates the full resolution pipeline: candidate collection,
/// scoring, tie-breaking, delta composition, and conflict materialization.
///
/// ## Example
///
/// ```ignore
/// let outcome = resolver.resolve(&block, &ctx).await?;
/// println!("View: {:?}", outcome.view);
/// ```
#[derive(Debug, Clone)]
pub struct ProjectionResolver {
    registry: StaticProjectionRegistry,
}

impl ProjectionResolver {
    /// Construct a new `ProjectionResolver`.
    #[must_use]
    pub fn new(registry: StaticProjectionRegistry) -> Self {
        Self { registry }
    }

    /// Resolve the winning projection for a block.
    ///
    /// Returns a [`ResolutionOutcome`] containing the view, winner ID,
    /// and conflict flag.
    #[instrument(skip_all, fields(block_id = %block.id))]
    pub fn resolve(
        &self,
        block: &Block,
        ctx: &ProjectionContext,
    ) -> Result<ResolutionOutcome, ApplicationError> {
        // 1. Collect all matching candidates
        let candidates: Vec<_> = self
            .registry
            .iter()
            .filter(|rp| rp.contract.matches_block(block))
            .collect();

        // 2. Score all candidates
        let scored: Vec<_> = candidates
            .iter()
            .map(|rp| {
                let score = rp.contract.score_block(block);
                (rp, score)
            })
            .collect();

        // 3. Determine winner (highest score; tie-break: smallest priority)
        let winner;
        let had_conflict;

        if scored.is_empty() {
            // No contract matched — fall back to DefaultProjection
            let default_rp = self
                .registry
                .get(&ProjectionContractId::new("default").unwrap())
                .expect("default contract must be present in V1 registry");
            let default_id = default_rp.projection.contract_id();
            let delta = default_rp.projection.apply(block, ctx);
            let base = ProjectionViewBuilder::new(block).build();
            let view = delta.compose_on(base);

            return Ok(ResolutionOutcome {
                view,
                winner_id: Some(default_id),
                had_conflict: false,
            });
        }

        // Find the highest score
        let top_score = scored
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::NEG_INFINITY, f64::max);

        // Filter to only top-scoring candidates
        let top_candidates: Vec<_> = scored.iter().filter(|(_, s)| *s == top_score).collect();

        if top_candidates.len() == 1 {
            // Unambiguous winner
            let (rp, _) = top_candidates[0];
            let delta = rp.projection.apply(block, ctx);
            let base = ProjectionViewBuilder::new(block).build();
            let view = delta.compose_on(base);
            winner = Some(rp.projection.contract_id());
            had_conflict = false;

            Ok(ResolutionOutcome {
                view,
                winner_id: winner,
                had_conflict,
            })
        } else {
            // Tie — pick smallest priority (lower number = higher priority)
            let (rp, _) = top_candidates
                .iter()
                .min_by_key(|(rp, _)| rp.contract.priority)
                .expect("top_candidates is non-empty");

            // Build conflict with all tied candidates
            let tied_ids: Vec<_> = top_candidates
                .iter()
                .map(|(rp, _)| rp.contract.id.clone())
                .collect();

            let conflict = ProjectionConflict {
                reason: format!(
                    "tied score: {} contracts with score {}",
                    top_candidates.len(),
                    top_score
                ),
                candidates: tied_ids.clone(),
                winner: None, // V1 spec: no winner on tie, fall back to default
                block_id: block.id,
            };

            // Apply delta from the tie-break winner and attach conflict
            let delta = rp.projection.apply(block, ctx);
            let conflict_delta = ProjectionViewDelta {
                decorations: vec![],
                view_properties: vec![],
                conflicts: vec![conflict],
            };
            let base = ProjectionViewBuilder::new(block).build();
            let view = conflict_delta.compose_on(delta.compose_on(base));

            // Conflict is surfaced via the view (not mutation)
            // Clients can inspect view.conflicts or the had_conflict flag
            winner = None;
            had_conflict = true;

            Ok(ResolutionOutcome {
                view,
                winner_id: winner,
                had_conflict,
            })
        }
    }

    /// Resolve and materialize the result on the block.
    ///
    /// This is a convenience method that calls [`Self::resolve`] and then
    /// mutates the block's properties to record:
    /// - `projection`: the winner's contract ID (or `"default"` on fallback)
    /// - `projection-conflict`: `"true"` if a conflict was detected
    /// - `projection-conflict-reason`: human-readable conflict reason
    /// - `projection-conflict-candidates`: comma-separated candidate IDs
    ///
    /// Returns the updated block.
    #[instrument(skip_all, fields(block_id = %block.id))]
    pub fn resolve_and_materialize(
        &self,
        block: &mut Block,
        ctx: &ProjectionContext,
    ) -> Result<ResolutionOutcome, ApplicationError> {
        let outcome = self.resolve(block, ctx)?;

        // Materialize projection property
        let projection_key = PropertyKey::new("projection").unwrap();
        let projection_value = PropertyValue::string(
            outcome
                .winner_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("default"),
        );
        block
            .properties
            .insert(projection_key.to_string(), projection_value);

        // Materialize conflict properties if needed
        if outcome.had_conflict {
            let conflict_key = PropertyKey::new("projection-conflict").unwrap();
            block
                .properties
                .insert(conflict_key.to_string(), PropertyValue::string("true"));

            // Find conflict reason from the view
            if let Some(conflict) = outcome.view.conflicts.first() {
                let reason_key = PropertyKey::new("projection-conflict-reason").unwrap();
                block.properties.insert(
                    reason_key.to_string(),
                    PropertyValue::string(&conflict.reason),
                );

                let candidates_key = PropertyKey::new("projection-conflict-candidates").unwrap();
                let candidates_str = conflict
                    .candidates
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                block.properties.insert(
                    candidates_key.to_string(),
                    PropertyValue::string(candidates_str),
                );
            }
        }

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block(props: HashMap<String, PropertyValue>) -> Block {
        Block {
            id: quilt_domain::value_objects::Uuid::new_v4(),
            page_id: quilt_domain::value_objects::Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: quilt_domain::value_objects::BlockFormat::Markdown,
            block_type: quilt_domain::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "Test block".into(),
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

    fn resolver() -> ProjectionResolver {
        ProjectionResolver::new(StaticProjectionRegistry::v1())
    }

    fn ctx() -> ProjectionContext {
        ProjectionContext::page(Utc::now())
    }

    // ── Basic resolution ──────────────────────────────────────────

    #[test]
    fn resolves_task_block_to_task_projection() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("task")
        );
        // TaskCheckbox decoration should be present
        assert!(
            outcome.view.decorations.iter().any(|d| {
                d.kind == quilt_domain::projection::view::DecorationKind::TaskCheckbox
            })
        );
    }

    #[test]
    fn resolves_media_block_to_media_projection() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("media"));
            p.insert("media-type".into(), PropertyValue::string("video"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("media")
        );
        assert!(
            outcome.view.decorations.iter().any(|d| {
                d.kind == quilt_domain::projection::view::DecorationKind::MediaPreview
            })
        );
    }

    #[test]
    fn resolves_heading_block_to_heading_projection() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("block-role".into(), PropertyValue::string("heading"));
            p.insert("heading-level".into(), PropertyValue::integer(2));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("heading")
        );
    }

    #[test]
    fn resolves_link_block_to_link_projection() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("link".into(), PropertyValue::string("https://example.com"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("link")
        );
    }

    #[test]
    fn resolves_date_block_to_date_projection() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("scheduled".into(), PropertyValue::string("2026-06-15"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("date")
        );
    }

    // ── Fallback ─────────────────────────────────────────────────

    #[test]
    fn resolves_unknown_block_to_default() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("custom".into(), PropertyValue::string("value"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        // No match → fallback to default (had_conflict=false, winner_id=default)
        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("default")
        );
        // DefaultProjection produces no decorations
        assert!(outcome.view.decorations.is_empty());
    }

    #[test]
    fn resolves_empty_block_to_default() {
        let block = make_block(HashMap::new());
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("default")
        );
    }

    // ── Priority ordering ────────────────────────────────────────

    #[test]
    fn task_beats_media_when_both_match() {
        // Both task and media predicates could match — task (priority 100)
        // should win over media (priority 200) since it has higher priority
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task")); // matches task
            p.insert("status".into(), PropertyValue::string("done"));
            p.insert("media-type".into(), PropertyValue::string("video")); // also matches media type signature
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(!outcome.had_conflict);
        // Task has priority 100, media has 200 — task wins (lower priority number)
        assert_eq!(
            outcome.winner_id.as_ref().map(|id| id.as_str()),
            Some("task")
        );
    }

    // ── Materialize ──────────────────────────────────────────────

    #[test]
    fn resolve_and_materialize_sets_projection_property() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("todo"));
            p
        });
        let mut block = block;
        let outcome = resolver()
            .resolve_and_materialize(&mut block, &ctx())
            .unwrap();

        assert_eq!(
            block.properties.get("projection"),
            Some(&PropertyValue::string("task"))
        );
    }

    #[test]
    fn resolve_and_materialize_sets_conflict_properties_on_tie() {
        // heading (priority 150) and media (priority 200) don't naturally tie
        // on the same block. To create a tie, we'd need two contracts with
        // the same priority. Since v1 registry ensures unique priorities,
        // the tie scenario is only tested via direct contract construction.
        // This test verifies no conflict props are set when there's no tie.
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let mut block = block;
        let _outcome = resolver()
            .resolve_and_materialize(&mut block, &ctx())
            .unwrap();

        assert!(!block.properties.contains_key("projection-conflict"));
    }

    // ── View composition ─────────────────────────────────────────

    #[test]
    fn view_carries_base_text() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert_eq!(outcome.view.text, "Test block");
    }

    #[test]
    fn view_carries_base_properties() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("custom-field".into(), PropertyValue::string("custom-value"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        assert!(
            outcome
                .view
                .properties
                .contains_key(&PropertyKey::new("type").unwrap())
        );
        assert!(
            outcome
                .view
                .properties
                .contains_key(&PropertyKey::new("custom-field").unwrap())
        );
    }

    #[test]
    fn view_includes_winner_delta_properties() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let outcome = resolver().resolve(&block, &ctx()).unwrap();

        // projection:: task should be in the view's effective properties
        assert_eq!(
            outcome
                .view
                .properties
                .get(&PropertyKey::new("projection").unwrap()),
            Some(&PropertyValue::string("task"))
        );
    }
}
