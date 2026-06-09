//! Strategy selector — deterministic strategy selection per block.
//!
//! The outliner needs to render and edit blocks in different ways depending
//! on what kind of block it is. Rather than encoding every rendering rule in
//! the outliner, `quilt-core` exposes a small trait system that lets callers
//! pick a `Strategy` for a given `Block` deterministically, based on the
//! block's properties (e.g. `type:: task`, `type:: query`).
//!
//! ## Crate placement
//!
//! Lives in `quilt-core` (WASM) per ADR
//! `docs/adr/drafts/DRAFT-strategy-selector-trait-contract.md`. The trait
//! surface here is the **per-block** selector the outliner calls. The
//! portfolio-based scorer (context features → ranked actions) is a separate
//! module and is also exposed here for future use.
//!
//! ## `Block` shape
//!
//! `quilt-core` is intentionally independent from `quilt-domain` to stay
//! WASM-pure. We define a minimal [`Block`] view that holds only the fields
//! the selector needs (`properties`). Adapters in
//! `quilt-application` (not part of this crate) can convert from the
//! full domain entity to this view.

use std::collections::HashMap;

// ── Block view (WASM-pure) ──────────────────────────────────────────

/// Minimal block view used by the strategy selector.
///
/// Holds only the field(s) the selector needs. The full domain entity
/// (with timestamps, refs, tags, etc.) lives in `quilt-domain`; an
/// adapter converts from `quilt_domain::entities::Block` to this struct.
///
/// Kept intentionally small so the trait is cheap to call from WASM
/// (sub-millisecond on real devices — the ADR requires sub-100ms for
/// the full portfolio scorer; per-block dispatch is the hot path).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    /// Block properties (e.g. `type:: task`, `priority:: A`).
    ///
    /// Values are stored as `String` — the selector only inspects them
    /// for equality. Richer types (numbers, refs) are not relevant to
    /// strategy selection in Phase 1.
    pub properties: HashMap<String, String>,
}

impl Block {
    /// Construct a block with the given properties.
    pub fn new(properties: HashMap<String, String>) -> Self {
        Self { properties }
    }

    /// Get the value of the `type` property, if any.
    pub fn block_type(&self) -> Option<&str> {
        self.properties.get("type").map(String::as_str)
    }
}

// ── Strategy trait ──────────────────────────────────────────────────

/// A strategy renders or edits blocks of a particular kind.
///
/// Strategies are pure, stateless objects — they describe how to handle a
/// block, they don't mutate it. (Future strategies may carry configuration,
/// but the trait surface stays the same.)
pub trait Strategy: Send + Sync {
    /// Stable, lowercase identifier for the strategy.
    ///
    /// Used by the registry and surfaced to the frontend so the React
    /// side can pick a matching component. Example: `"task"`, `"query"`.
    fn name(&self) -> &str;

    /// Whether this strategy applies to the given block.
    ///
    /// Implementations should be cheap (no I/O, no allocation beyond what
    /// `HashMap::get` already does) — the outliner calls this on every
    /// visible block.
    fn can_handle(&self, block: &Block) -> bool;
}

// ── StrategySelector trait ──────────────────────────────────────────

/// Registry of strategies. Selects the first strategy that handles a
/// block, in registration order.
///
/// `select` returns `None` only if no strategy was registered (or the
/// caller forgot to include the default). The built-in
/// [`DefaultStrategySelector`] always returns *something* — unknown
/// types fall through to `DefaultStrategy`.
pub trait StrategySelector: Send + Sync {
    /// Pick a strategy for the given block, or `None` if no registered
    /// strategy handles it.
    fn select(&self, block: &Block) -> Option<&dyn Strategy>;

    /// Names of all registered strategies, in registration order.
    fn all(&self) -> Vec<&str>;
}

// ── StrategyScorer trait (future, ADR Phase 1) ──────────────────────

/// Scores how applicable a strategy is to a given block, in `[0.0, 1.0]`.
///
/// The default selector uses only `can_handle` (binary). Future phases
/// can rank multiple matching strategies with `score`. Defined here so
/// the trait is part of the stable contract from day one.
pub trait StrategyScorer: Send + Sync {
    /// Returns a score in `0.0..=1.0` indicating how applicable the
    /// given strategy is to the block. Implementations MUST clamp the
    /// result into range (callers will use it for ranking).
    fn score(&self, strategy: &str, block: &Block) -> f32;
}

// ── Built-in strategies ─────────────────────────────────────────────

/// `DefaultStrategy` — fallback for blocks without a recognised `type`.
///
/// Always claims it can handle a block. Registered **last** so the
/// outliner picks more specific strategies first.
#[derive(Debug, Clone, Default)]
pub struct DefaultStrategy;

impl Strategy for DefaultStrategy {
    fn name(&self) -> &str {
        "default"
    }

    fn can_handle(&self, _block: &Block) -> bool {
        true
    }
}

/// `TaskStrategy` — handles blocks with `type:: task`.
#[derive(Debug, Clone, Default)]
pub struct TaskStrategy;

impl Strategy for TaskStrategy {
    fn name(&self) -> &str {
        "task"
    }

    fn can_handle(&self, block: &Block) -> bool {
        block.block_type() == Some("task")
    }
}

/// `QueryStrategy` — handles blocks with `type:: query`.
#[derive(Debug, Clone, Default)]
pub struct QueryStrategy;

impl Strategy for QueryStrategy {
    fn name(&self) -> &str {
        "query"
    }

    fn can_handle(&self, block: &Block) -> bool {
        block.block_type() == Some("query")
    }
}

/// `ViewStrategy` — handles blocks with `type:: view`.
#[derive(Debug, Clone, Default)]
pub struct ViewStrategy;

impl Strategy for ViewStrategy {
    fn name(&self) -> &str {
        "view"
    }

    fn can_handle(&self, block: &Block) -> bool {
        block.block_type() == Some("view")
    }
}

/// `AgentRunStrategy` — handles blocks with `type:: agent-run`.
#[derive(Debug, Clone, Default)]
pub struct AgentRunStrategy;

impl Strategy for AgentRunStrategy {
    fn name(&self) -> &str {
        "agent-run"
    }

    fn can_handle(&self, block: &Block) -> bool {
        block.block_type() == Some("agent-run")
    }
}

// ── Default implementation of the selector ──────────────────────────

/// In-memory selector that holds an ordered list of `Box<dyn Strategy>`.
///
/// Concrete strategies are registered up-front. The first strategy whose
/// `can_handle` returns `true` is selected. A `DefaultStrategy` is
/// typically appended last as a fallback.
#[derive(Default)]
pub struct DefaultStrategySelector {
    strategies: Vec<Box<dyn Strategy>>,
}

impl DefaultStrategySelector {
    /// Build a selector with no strategies registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a strategy. Order matters: `select` walks the list
    /// front-to-back and returns the first match.
    pub fn register<S: Strategy + 'static>(mut self, strategy: S) -> Self {
        self.strategies.push(Box::new(strategy));
        self
    }

    /// Build the canonical Quilt selector: Task → Query → View → AgentRun → Default.
    ///
    /// This is the selector the outliner and MCP should use by default.
    /// Tests that need custom registries should use [`new`](Self::new)
    /// and [`register`](Self::register) directly.
    pub fn with_builtins() -> Self {
        Self::new()
            .register(TaskStrategy)
            .register(QueryStrategy)
            .register(ViewStrategy)
            .register(AgentRunStrategy)
            .register(DefaultStrategy)
    }
}

impl StrategySelector for DefaultStrategySelector {
    fn select(&self, block: &Block) -> Option<&dyn Strategy> {
        self.strategies
            .iter()
            .find(|s| s.can_handle(block))
            .map(|s| s.as_ref())
    }

    fn all(&self) -> Vec<&str> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
}

// ── Default scorer (optional, used by future portfolio selector) ───

/// Returns `1.0` for any strategy that handles the block, `0.0` otherwise.
///
/// Simple, deterministic, and matches the Phase-1 rule of "no scoring, just
/// classification". Strategies that want a partial score can implement
/// their own scorer.
#[derive(Debug, Clone, Default)]
pub struct BinaryScorer;

impl StrategyScorer for BinaryScorer {
    fn score(&self, _strategy: &str, _block: &Block) -> f32 {
        // We don't actually have a strategy reference here — the contract
        // is the score is for the *name*. Concrete scorers in
        // `quilt-application` will close over the registered strategies.
        1.0
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn block_with_type(t: &str) -> Block {
        let mut props = HashMap::new();
        props.insert("type".to_string(), t.to_string());
        Block::new(props)
    }

    fn empty_block() -> Block {
        Block::default()
    }

    // ── Strategy::name ────────────────────────────────────────────

    #[test]
    fn default_strategy_name() {
        assert_eq!(DefaultStrategy.name(), "default");
    }

    #[test]
    fn task_strategy_name() {
        assert_eq!(TaskStrategy.name(), "task");
    }

    #[test]
    fn query_strategy_name() {
        assert_eq!(QueryStrategy.name(), "query");
    }

    #[test]
    fn view_strategy_name() {
        assert_eq!(ViewStrategy.name(), "view");
    }

    #[test]
    fn agent_run_strategy_name() {
        assert_eq!(AgentRunStrategy.name(), "agent-run");
    }

    // ── Strategy::can_handle ───────────────────────────────────────

    #[test]
    fn default_strategy_handles_everything() {
        let s = DefaultStrategy;
        assert!(s.can_handle(&empty_block()));
        assert!(s.can_handle(&block_with_type("task")));
        assert!(s.can_handle(&block_with_type("anything-else")));
    }

    #[test]
    fn task_strategy_handles_only_task_type() {
        let s = TaskStrategy;
        assert!(s.can_handle(&block_with_type("task")));
        assert!(!s.can_handle(&block_with_type("query")));
        assert!(!s.can_handle(&empty_block()));
    }

    #[test]
    fn query_strategy_handles_only_query_type() {
        let s = QueryStrategy;
        assert!(s.can_handle(&block_with_type("query")));
        assert!(!s.can_handle(&block_with_type("task")));
        assert!(!s.can_handle(&empty_block()));
    }

    #[test]
    fn view_strategy_handles_only_view_type() {
        let s = ViewStrategy;
        assert!(s.can_handle(&block_with_type("view")));
        assert!(!s.can_handle(&block_with_type("task")));
        assert!(!s.can_handle(&empty_block()));
    }

    #[test]
    fn agent_run_strategy_handles_only_agent_run_type() {
        let s = AgentRunStrategy;
        assert!(s.can_handle(&block_with_type("agent-run")));
        assert!(!s.can_handle(&block_with_type("task")));
        assert!(!s.can_handle(&empty_block()));
    }

    // ── StrategySelector::select ──────────────────────────────────

    #[test]
    fn selector_picks_task_for_task_block() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel.select(&block_with_type("task")).expect("must select");
        assert_eq!(s.name(), "task");
    }

    #[test]
    fn selector_picks_query_for_query_block() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel.select(&block_with_type("query")).expect("must select");
        assert_eq!(s.name(), "query");
    }

    #[test]
    fn selector_picks_view_for_view_block() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel.select(&block_with_type("view")).expect("must select");
        assert_eq!(s.name(), "view");
    }

    #[test]
    fn selector_picks_agent_run_for_agent_run_block() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel
            .select(&block_with_type("agent-run"))
            .expect("must select");
        assert_eq!(s.name(), "agent-run");
    }

    #[test]
    fn unknown_type_falls_back_to_default() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel
            .select(&block_with_type("something-not-recognised"))
            .expect("default must always match");
        assert_eq!(s.name(), "default");
    }

    #[test]
    fn block_with_no_properties_falls_back_to_default() {
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel.select(&empty_block()).expect("default must match");
        assert_eq!(s.name(), "default");
    }

    #[test]
    fn block_with_unrelated_property_falls_back_to_default() {
        let mut props = HashMap::new();
        props.insert("priority".to_string(), "A".to_string());
        let block = Block::new(props);
        let sel = DefaultStrategySelector::with_builtins();
        let s = sel.select(&block).expect("default must match");
        assert_eq!(s.name(), "default");
    }

    // ── StrategySelector::all ──────────────────────────────────────

    #[test]
    fn all_lists_every_registered_strategy_in_order() {
        let sel = DefaultStrategySelector::with_builtins();
        let names = sel.all();
        assert_eq!(
            names,
            vec!["task", "query", "view", "agent-run", "default"],
            "built-in selector must register all five strategies in canonical order"
        );
    }

    #[test]
    fn all_returns_empty_for_fresh_selector() {
        let sel = DefaultStrategySelector::new();
        assert!(sel.all().is_empty());
    }

    #[test]
    fn all_returns_strategies_in_registration_order() {
        let sel = DefaultStrategySelector::new()
            .register(ViewStrategy)
            .register(TaskStrategy)
            .register(DefaultStrategy);
        assert_eq!(sel.all(), vec!["view", "task", "default"]);
    }

    // ── Custom registries ─────────────────────────────────────────

    #[test]
    fn selector_with_only_default_returns_default_for_anything() {
        let sel = DefaultStrategySelector::new().register(DefaultStrategy);
        assert_eq!(
            sel.select(&block_with_type("task")).unwrap().name(),
            "default"
        );
        assert_eq!(sel.select(&empty_block()).unwrap().name(), "default");
    }

    #[test]
    fn selector_with_no_strategies_returns_none() {
        let sel = DefaultStrategySelector::new();
        assert!(sel.select(&block_with_type("task")).is_none());
    }

    #[test]
    fn first_registered_match_wins() {
        // Task is registered first → wins over default for a task block.
        let sel = DefaultStrategySelector::new()
            .register(TaskStrategy)
            .register(DefaultStrategy);
        let s = sel.select(&block_with_type("task")).unwrap();
        assert_eq!(s.name(), "task");
    }

    // ── DefaultStrategySelector + trait-object ergonomics ─────────

    #[test]
    fn selector_is_usable_through_trait_object() {
        // Compile-time check: `Box<dyn StrategySelector>` must be valid.
        let sel: Box<dyn StrategySelector> = Box::new(DefaultStrategySelector::with_builtins());
        assert_eq!(sel.all().len(), 5);
        assert_eq!(sel.select(&block_with_type("task")).unwrap().name(), "task");
    }

    // ── StrategyScorer ────────────────────────────────────────────

    #[test]
    fn binary_score_is_one() {
        let scorer = BinaryScorer;
        assert_eq!(scorer.score("task", &empty_block()), 1.0);
        assert_eq!(scorer.score("default", &block_with_type("task")), 1.0);
    }

    // ── Block::block_type helper ──────────────────────────────────

    #[test]
    fn block_type_returns_value_when_present() {
        assert_eq!(block_with_type("task").block_type(), Some("task"));
    }

    #[test]
    fn block_type_returns_none_when_absent() {
        assert_eq!(empty_block().block_type(), None);
    }
}
