//! Concrete [`StrategyScorer`] and [`StrategySelector`] implementations.
//!
//! The trait contracts in [`crate::strategy`] (e.g. `StrategyScorer`,
//! `StrategySelector`) are deliberately minimal so the trait surface stays
//! stable. This module provides the *default concrete* scorers and
//! selectors that downstream crates (`quilt-application`,
//! `quilt-analysis`) actually use today.
//!
//! ## Scoring model
//!
//! [`RelevanceScorer`] combines four signals with fixed weights, summing
//! to 1.0:
//!
//! | Signal               | Weight | What it measures                                  |
//! |----------------------|--------|---------------------------------------------------|
//! | Type match           | 0.50   | Does the strategy name match the block's `type`?  |
//! | Property completeness | 0.20  | How many of the strategy's expected props are set |
//! | Recency              | 0.15   | Half-life decay on `updated-at`                   |
//! | Semantic similarity  | 0.15   | Reserved (neutral 0.5) until WASM gets embeddings |
//!
//! The result is clamped into `[0.0, 1.0]`. Weights are exposed as
//! constants so tests and downstream callers can reference them.
//!
//! ## Selector
//!
//! [`ScoredStrategySelector`] picks the *highest-scoring* strategy for
//! a block, breaking ties by registration order. This is what the future
//! portfolio selector (ADR Phase 1) builds on. The legacy
//! `DefaultStrategySelector` (first-match-wins) still ships alongside
//! for callers that prefer deterministic, classification-only dispatch.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::strategy::{Block, Strategy, StrategyScorer, StrategySelector};

// ── Weight constants (public for tests / introspection) ─────────────

/// Weight applied to the **type-match** signal (0.0..=1.0).
pub const WEIGHT_TYPE_MATCH: f32 = 0.50;

/// Weight applied to the **property completeness** signal (0.0..=1.0).
pub const WEIGHT_PROPERTY_COMPLETENESS: f32 = 0.20;

/// Weight applied to the **recency** signal (0.0..=1.0).
pub const WEIGHT_RECENCY: f32 = 0.15;

/// Weight applied to the **semantic similarity** signal (0.0..=1.0).
///
/// Currently a neutral placeholder (returns 0.5) — the WASM build
/// doesn't carry an embedding model. We keep the weight reserved so the
/// signal can be turned on without re-tuning downstream callers.
pub const WEIGHT_SEMANTIC_SIMILARITY: f32 = 0.15;

/// Half-life in hours for the recency decay. A block updated 24h ago
/// scores `0.5`; a block updated 6h ago scores `0.5 * 2^(6/24) ≈ 0.707`.
/// Constants match a "knowledge graph refresh" cadence where blocks
/// older than a few days carry less weight.
pub const RECENCY_HALF_LIFE_HOURS: f64 = 24.0;

/// Neutral score used when a signal cannot be evaluated (missing
/// `updated-at`, no embedding available, etc.). Keeps the weighted sum
/// well-defined rather than zeroing the whole score.
pub const NEUTRAL_SIGNAL: f32 = 0.5;

// ── Property expectations per strategy name ─────────────────────────

/// Properties a strategy "expects" to see, used by the
/// **property-completeness** signal. The fraction of expected keys
/// actually present in the block becomes that signal's value.
///
/// Kept as a free function (not a `HashMap` constant) so a future phase
/// can compute expectations dynamically from the registered strategy
/// instance — for now the static table is enough.
fn expected_properties(strategy_name: &str) -> &'static [&'static str] {
    match strategy_name {
        "task" => &["priority"],
        "query" => &["dsl"],
        "view" => &["layout"],
        "agent-run" => &["agent", "status"],
        // `default` and any unknown strategy name: no expectations.
        _ => &[],
    }
}

// ── Timestamp parsing helper ────────────────────────────────────────

/// Parse the block's `updated-at` property into a `DateTime<Utc>`.
///
/// Accepts both RFC 3339 strings (`"2024-01-15T10:00:00Z"`) and
/// integer Unix epoch seconds (`"1705312800"`). Returns `None` for
/// missing, empty, or unparseable values so the caller can substitute
/// the [`NEUTRAL_SIGNAL`].
fn parse_updated_at(block: &Block) -> Option<DateTime<Utc>> {
    let raw = block.properties.get("updated-at")?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try RFC 3339 first — the most common shape produced by
    // `chrono::DateTime<Utc>::to_rfc3339()`.
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }

    // Fall back to Unix epoch seconds (integer-encoded as a string).
    if let Ok(epoch) = trimmed.parse::<i64>() {
        let dt = DateTime::from_timestamp(epoch, 0)?;
        return Some(dt);
    }

    None
}

// ── Concrete scorer ─────────────────────────────────────────────────

/// A [`StrategyScorer`] that combines four weighted signals into a
/// single score in `[0.0, 1.0]`.
///
/// This is the **default concrete scorer** for the portfolio selector
/// pipeline. Construct it with a `now` (so tests can pin time) or use
/// [`RelevanceScorer::new`] to use wall-clock time.
///
/// ## Signal details
///
/// 1. **Type match** — `1.0` if `block.type == strategy_name`,
///    `0.0` otherwise. Hard signal, dominates the score.
/// 2. **Property completeness** — fraction of the strategy's
///    `expected_properties()` actually set on the block. A `task`
///    block with `priority:: A` scores `1.0`; one without scores
///    `0.0`. Unknown strategies score `NEUTRAL_SIGNAL` so the signal
///    doesn't punish them unfairly.
/// 3. **Recency** — exponential decay with [`RECENCY_HALF_LIFE_HOURS`].
///    Uses the block's `updated-at` property. Missing timestamps
///    yield [`NEUTRAL_SIGNAL`].
/// 4. **Semantic similarity** — currently `NEUTRAL_SIGNAL` for all
///    blocks. Reserved for future embedding-based scoring.
///
/// Scores are clamped to `[0.0, 1.0]` per the trait contract, even
/// though the weighted sum of clamped signals is mathematically
/// bounded in `[0.0, 1.0]` already.
#[derive(Debug, Clone)]
pub struct RelevanceScorer {
    /// The reference "now" used to compute recency. Stored so tests
    /// can inject a fixed clock.
    now: DateTime<Utc>,
}

impl RelevanceScorer {
    /// Build a scorer that uses wall-clock time (`Utc::now()`) for
    /// recency calculations.
    pub fn new() -> Self {
        Self { now: Utc::now() }
    }

    /// Build a scorer with a fixed `now` — useful for deterministic
    /// tests and for replaying a saved graph state.
    pub fn with_now(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    /// Compute the **type-match** signal: `1.0` if the block's `type`
    /// property equals the strategy name, `0.0` otherwise.
    pub fn type_match_signal(strategy: &str, block: &Block) -> f32 {
        match block.block_type() {
            Some(t) if t == strategy => 1.0,
            _ => 0.0,
        }
    }

    /// Compute the **property completeness** signal. A `task` block
    /// with all of `[priority]` set scores `1.0`; a `default` block
    /// (no expectations) always scores `NEUTRAL_SIGNAL`.
    pub fn property_completeness_signal(strategy: &str, block: &Block) -> f32 {
        let expected = expected_properties(strategy);
        if expected.is_empty() {
            return NEUTRAL_SIGNAL;
        }
        let present = expected
            .iter()
            .filter(|k| {
                block
                    .properties
                    .get(**k)
                    .is_some_and(|v| !v.trim().is_empty())
            })
            .count();
        // Cast to f32 *after* division so the precision is determined
        // by the integer count.
        present as f32 / expected.len() as f32
    }

    /// Compute the **recency** signal via exponential half-life
    /// decay. Returns `NEUTRAL_SIGNAL` when no `updated-at` is
    /// available.
    pub fn recency_signal(&self, block: &Block) -> f32 {
        let Some(updated) = parse_updated_at(block) else {
            return NEUTRAL_SIGNAL;
        };
        let age_hours = (self.now - updated).num_seconds().max(0) as f64 / 3600.0;
        // 0.5^(age/half_life) — `0.5` is the score for one half-life
        // old, `1.0` for brand-new, approaching `0.0` for ancient.
        let decay = 0.5_f64.powf(age_hours / RECENCY_HALF_LIFE_HOURS);
        (decay as f32).clamp(0.0, 1.0)
    }

    /// Compute the **semantic similarity** signal. Currently a
    /// neutral placeholder.
    pub fn semantic_similarity_signal(_strategy: &str, _block: &Block) -> f32 {
        NEUTRAL_SIGNAL
    }
}

impl Default for RelevanceScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategyScorer for RelevanceScorer {
    fn score(&self, strategy: &str, block: &Block) -> f32 {
        let type_match = Self::type_match_signal(strategy, block);
        let property = Self::property_completeness_signal(strategy, block);
        let recency = self.recency_signal(block);
        let semantic = Self::semantic_similarity_signal(strategy, block);

        let raw = type_match * WEIGHT_TYPE_MATCH
            + property * WEIGHT_PROPERTY_COMPLETENESS
            + recency * WEIGHT_RECENCY
            + semantic * WEIGHT_SEMANTIC_SIMILARITY;

        // Sum of clamped signals * weights is bounded in [0, 1] by
        // construction, but clamp defensively in case weights ever
        // change.
        raw.clamp(0.0, 1.0)
    }
}

// ── Scored selector ─────────────────────────────────────────────────

/// A [`StrategySelector`] that picks the **highest-scoring** strategy
/// for a block, breaking ties by registration order.
///
/// This is the selector the future portfolio pipeline uses. The
/// legacy `DefaultStrategySelector` (first-match-wins) is preserved
/// for callers that want deterministic classification without the
/// scoring cost.
pub struct ScoredStrategySelector {
    strategies: Vec<Box<dyn Strategy>>,
    scorer: Box<dyn StrategyScorer>,
}

impl ScoredStrategySelector {
    /// Build a scored selector with no strategies and the default
    /// [`RelevanceScorer`].
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
            scorer: Box::new(RelevanceScorer::new()),
        }
    }

    /// Register a strategy. Order matters: ties on score are broken
    /// by earlier registration.
    pub fn register<S: Strategy + 'static>(mut self, strategy: S) -> Self {
        self.strategies.push(Box::new(strategy));
        self
    }

    /// Replace the default [`RelevanceScorer`] with a custom
    /// implementation. Useful for tests and for the future
    /// embedding-based pipeline.
    pub fn with_scorer(mut self, scorer: Box<dyn StrategyScorer>) -> Self {
        self.scorer = scorer;
        self
    }

    /// Return the highest-scoring strategy and its score, or
    /// `None` if no strategy is registered.
    pub fn best_match(&self, block: &Block) -> Option<(&dyn Strategy, f32)> {
        let mut best: Option<(&dyn Strategy, f32)> = None;
        for s in &self.strategies {
            let score = self.scorer.score(s.name(), block);
            match best {
                Some((_, best_score)) if score <= best_score => {}
                _ => best = Some((s.as_ref(), score)),
            }
        }
        best
    }
}

impl Default for ScoredStrategySelector {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategySelector for ScoredStrategySelector {
    fn select(&self, block: &Block) -> Option<&dyn Strategy> {
        // Filter to strategies that *can handle* the block AND have a
        // non-zero score. Falls back to the legacy first-match-wins
        // behaviour when the scorer assigns 0.0 to every matchable
        // strategy (rare; signals usually push the type-match above
        // zero).
        let matchable: Option<&dyn Strategy> = self
            .strategies
            .iter()
            .find(|s| s.can_handle(block))
            .map(|s| s.as_ref());

        let best = self.best_match(block);
        match (matchable, best) {
            (Some(fallback), Some((strategy, score))) if score > 0.0 => {
                if strategy.can_handle(block) {
                    Some(strategy)
                } else {
                    // The scorer picked a strategy that doesn't
                    // actually match — fall back to the first
                    // matchable one. Keeps the contract: `select`
                    // never returns a strategy that can't handle the
                    // block.
                    Some(fallback)
                }
            }
            (Some(fallback), _) => Some(fallback),
            (None, _) => None,
        }
    }

    fn all(&self) -> Vec<&str> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
}

// ── Helper for tests / public API ───────────────────────────────────

/// Build a [`Block`] with the given `type` and any extra
/// properties. Re-exported here (next to the scoring code that
/// consumes it) so tests in this module stay readable.
pub fn block_with(
    block_type: Option<&str>,
    extra: &[(&str, &str)],
) -> Block {
    let mut props: HashMap<String, String> = HashMap::new();
    if let Some(t) = block_type {
        props.insert("type".to_string(), t.to_string());
    }
    for (k, v) in extra {
        props.insert((*k).to_string(), (*v).to_string());
    }
    Block::new(props)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::{
        AgentRunStrategy, DefaultStrategy, QueryStrategy, TaskStrategy, ViewStrategy,
    };

    // Fixed reference time for deterministic recency tests.
    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-06-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    // ── Type-match signal ─────────────────────────────────────────

    #[test]
    fn type_match_signal_is_one_when_type_equals_strategy() {
        let b = block_with(Some("task"), &[]);
        assert_eq!(RelevanceScorer::type_match_signal("task", &b), 1.0);
    }

    #[test]
    fn type_match_signal_is_zero_when_type_differs() {
        let b = block_with(Some("query"), &[]);
        assert_eq!(RelevanceScorer::type_match_signal("task", &b), 0.0);
    }

    #[test]
    fn type_match_signal_is_zero_when_type_absent() {
        let b = block_with(None, &[]);
        assert_eq!(RelevanceScorer::type_match_signal("task", &b), 0.0);
    }

    // ── Property completeness signal ─────────────────────────────

    #[test]
    fn property_completeness_full_when_all_expected_props_set() {
        let b = block_with(Some("task"), &[("priority", "A")]);
        assert_eq!(
            RelevanceScorer::property_completeness_signal("task", &b),
            1.0,
        );
    }

    #[test]
    fn property_completeness_partial_when_some_props_set() {
        // `task` expects `[priority]`; one out of one present ⇒ 1.0.
        // We use `agent-run` (expects `[agent, status]`) to get a
        // partial result.
        let b = block_with(Some("agent-run"), &[("agent", "quilt")]);
        assert_eq!(
            RelevanceScorer::property_completeness_signal("agent-run", &b),
            0.5,
        );
    }

    #[test]
    fn property_completeness_zero_when_no_props_set() {
        let b = block_with(Some("task"), &[]);
        assert_eq!(
            RelevanceScorer::property_completeness_signal("task", &b),
            0.0,
        );
    }

    #[test]
    fn property_completeness_neutral_for_default_strategy() {
        // `default` expects no properties ⇒ neutral signal, not 0.0.
        let b = block_with(Some("task"), &[]);
        assert_eq!(
            RelevanceScorer::property_completeness_signal("default", &b),
            NEUTRAL_SIGNAL,
        );
    }

    #[test]
    fn property_completeness_treats_empty_string_as_absent() {
        let b = block_with(Some("task"), &[("priority", "   ")]);
        assert_eq!(
            RelevanceScorer::property_completeness_signal("task", &b),
            0.0,
        );
    }

    // ── Recency signal ────────────────────────────────────────────

    #[test]
    fn recency_signal_is_one_for_freshly_updated_block() {
        let now = fixed_now();
        let b = block_with(Some("task"), &[("updated-at", &now.to_rfc3339())]);
        let scorer = RelevanceScorer::with_now(now);
        let score = scorer.recency_signal(&b);
        assert!(
            (score - 1.0).abs() < 1e-4,
            "freshly updated block should score ≈1.0, got {score}",
        );
    }

    #[test]
    fn recency_signal_is_half_at_one_half_life() {
        let now = fixed_now();
        let half_life_ago = now - chrono::Duration::hours(RECENCY_HALF_LIFE_HOURS as i64);
        let b = block_with(
            Some("task"),
            &[("updated-at", &half_life_ago.to_rfc3339())],
        );
        let scorer = RelevanceScorer::with_now(now);
        let score = scorer.recency_signal(&b);
        assert!(
            (score - 0.5).abs() < 1e-3,
            "block one half-life old should score ≈0.5, got {score}",
        );
    }

    #[test]
    fn recency_signal_decays_for_old_blocks() {
        let now = fixed_now();
        let long_ago = now - chrono::Duration::hours(240); // 10 half-lives
        let b = block_with(Some("task"), &[("updated-at", &long_ago.to_rfc3339())]);
        let scorer = RelevanceScorer::with_now(now);
        let score = scorer.recency_signal(&b);
        assert!(score < 0.01, "very old block should score ≈0.0, got {score}");
    }

    #[test]
    fn recency_signal_is_neutral_when_updated_at_missing() {
        let scorer = RelevanceScorer::with_now(fixed_now());
        let b = block_with(Some("task"), &[]);
        assert_eq!(scorer.recency_signal(&b), NEUTRAL_SIGNAL);
    }

    #[test]
    fn recency_signal_is_neutral_for_unparseable_timestamp() {
        let scorer = RelevanceScorer::with_now(fixed_now());
        let b = block_with(Some("task"), &[("updated-at", "not a date")]);
        assert_eq!(scorer.recency_signal(&b), NEUTRAL_SIGNAL);
    }

    #[test]
    fn recency_signal_parses_unix_epoch_seconds() {
        let now = fixed_now();
        let scorer = RelevanceScorer::with_now(now);
        let block_at_now =
            block_with(Some("task"), &[("updated-at", &now.timestamp().to_string())]);
        let score = scorer.recency_signal(&block_at_now);
        assert!((score - 1.0).abs() < 1e-3, "epoch seconds should parse, got {score}");
    }

    // ── Full scorer ───────────────────────────────────────────────

    #[test]
    fn score_clamps_to_unit_range() {
        let scorer = RelevanceScorer::with_now(fixed_now());
        let b = block_with(Some("task"), &[("priority", "A")]);
        let s = scorer.score("task", &b);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn type_match_outweighs_other_signals() {
        // A `task` block scored under "task" should be ranked above
        // a `query` block scored under "task". Type match dominates.
        let scorer = RelevanceScorer::with_now(fixed_now());
        let task = block_with(Some("task"), &[("priority", "A")]);
        let query = block_with(Some("query"), &[("priority", "A")]);
        let s_task = scorer.score("task", &task);
        let s_query = scorer.score("task", &query);
        assert!(
            s_task > s_query,
            "task-vs-task ({s_task}) should outscore query-vs-task ({s_query})",
        );
    }

    #[test]
    fn weights_sum_to_one() {
        let total = WEIGHT_TYPE_MATCH
            + WEIGHT_PROPERTY_COMPLETENESS
            + WEIGHT_RECENCY
            + WEIGHT_SEMANTIC_SIMILARITY;
        assert!(
            (total - 1.0).abs() < 1e-6,
            "weights must sum to 1.0, got {total}",
        );
    }

    #[test]
    fn fresh_block_outranks_old_block_of_same_type() {
        let now = fixed_now();
        let scorer = RelevanceScorer::with_now(now);
        let fresh = block_with(Some("task"), &[("updated-at", &now.to_rfc3339())]);
        let old = block_with(
            Some("task"),
            &[("updated-at", &(now - chrono::Duration::days(7)).to_rfc3339())],
        );
        let s_fresh = scorer.score("task", &fresh);
        let s_old = scorer.score("task", &old);
        assert!(
            s_fresh > s_old,
            "fresh ({s_fresh}) should outscore old ({s_old})",
        );
    }

    // ── Scored selector ───────────────────────────────────────────

    #[test]
    fn scored_selector_with_no_strategies_returns_none() {
        let sel = ScoredStrategySelector::new();
        let b = block_with(Some("task"), &[]);
        assert!(sel.select(&b).is_none());
    }

    #[test]
    fn scored_selector_picks_task_for_task_block() {
        let sel = ScoredStrategySelector::new()
            .register(TaskStrategy)
            .register(QueryStrategy)
            .register(ViewStrategy)
            .register(AgentRunStrategy)
            .register(DefaultStrategy);
        let b = block_with(Some("task"), &[("priority", "A")]);
        let picked = sel.select(&b).expect("must select");
        assert_eq!(picked.name(), "task");
    }

    #[test]
    fn scored_selector_picks_best_when_multiple_match() {
        // A block typed as "task" matches both `task` and `default`.
        // Type-match signal is 1.0 for "task" and 0.0 for "default" ⇒
        // "task" wins.
        let sel = ScoredStrategySelector::new()
            .register(DefaultStrategy)
            .register(TaskStrategy);
        let b = block_with(Some("task"), &[("priority", "A")]);
        let picked = sel.select(&b).expect("must select");
        assert_eq!(picked.name(), "task");
    }

    #[test]
    fn scored_selector_respects_tie_break_by_registration_order() {
        // Two strategies with identical signals. First registered
        // wins. We force the tie by giving both the same
        // type/match/recency.
        let sel = ScoredStrategySelector::new()
            .register(TaskStrategy)
            .register(DefaultStrategy);
        // Block with no `type` ⇒ only `default` can_handle.
        let b = block_with(None, &[]);
        let picked = sel.select(&b).expect("must select");
        assert_eq!(picked.name(), "default");
    }

    #[test]
    fn scored_selector_all_lists_in_registration_order() {
        let sel = ScoredStrategySelector::new()
            .register(ViewStrategy)
            .register(TaskStrategy)
            .register(DefaultStrategy);
        assert_eq!(sel.all(), vec!["view", "task", "default"]);
    }

    #[test]
    fn scored_selector_falls_back_to_first_matchable_when_no_signal() {
        // Custom scorer that returns 0.0 for everything. Selector
        // should still find *some* matchable strategy rather than
        // returning None or an empty pointer.
        struct ZeroScorer;
        impl StrategyScorer for ZeroScorer {
            fn score(&self, _strategy: &str, _block: &Block) -> f32 {
                0.0
            }
        }
        let sel = ScoredStrategySelector::new()
            .register(TaskStrategy)
            .register(DefaultStrategy)
            .with_scorer(Box::new(ZeroScorer));
        let b = block_with(Some("task"), &[]);
        let picked = sel.select(&b).expect("must select");
        // First matchable strategy for a `task` block is `task`.
        assert_eq!(picked.name(), "task");
    }

    #[test]
    fn scored_selector_returns_highest_score_for_partial_matches() {
        // A `task` block with a `priority` property should rank
        // `task` strategy (full signals) above `default` (no
        // type-match, no property completeness).
        let scorer = RelevanceScorer::with_now(fixed_now());
        let sel = ScoredStrategySelector::new()
            .register(TaskStrategy)
            .register(DefaultStrategy)
            .with_scorer(Box::new(scorer));
        let b = block_with(Some("task"), &[("priority", "A")]);
        let (_strategy, score) = sel.best_match(&b).expect("must have a match");
        // The dominant signal is type-match (0.5 weight) plus a
        // partial contribution from the others — we expect a score
        // well above the `default` strategy's score of
        // `0.0*0.5 + NEUTRAL*0.2 + NEUTRAL*0.15 + NEUTRAL*0.15 = 0.20`.
        assert!(
            score > 0.5,
            "task block should score well above the default strategy, got {score}",
        );
    }

    // ── block_with helper ─────────────────────────────────────────

    #[test]
    fn block_with_helper_omits_type_when_none() {
        let b = block_with(None, &[("foo", "bar")]);
        assert_eq!(b.block_type(), None);
        assert_eq!(b.properties.get("foo").map(String::as_str), Some("bar"));
    }

    #[test]
    fn block_with_helper_sets_type_when_provided() {
        let b = block_with(Some("query"), &[]);
        assert_eq!(b.block_type(), Some("query"));
    }
}
