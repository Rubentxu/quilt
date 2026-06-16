//! WASM bindings — exports for the React frontend

use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use crate::outliner::history::{self, HistoryStack, OutlinerCommand as HistoryCommand};
use crate::outliner::tree;
use crate::parser::inline::InlineParser;
use crate::types::{BlockDto, OutlinerCommand};

thread_local! {
    static BLOCKS: RefCell<HashMap<String, Vec<BlockDto>>> = RefCell::new(HashMap::new());
}

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn get_state(page_id: String) -> Result<JsValue, JsValue> {
    let (blocks, can_undo, can_redo) = BLOCKS.with(|store| {
        let store = store.borrow();
        let blocks = store.get(&page_id).cloned().unwrap_or_default();
        // Undo/redo disabled for now — will be re-added with proper history
        (blocks, false, false)
    });

    let state = serde_json::json!({
        "blocks": blocks,
        "canUndo": can_undo,
        "canRedo": can_redo,
        "stateHash": compute_hash(&blocks),
    });

    let json_str = serde_json::to_string(&state)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn load_page(page_id: String, blocks_json: JsValue) -> Result<JsValue, JsValue> {
    let blocks: Vec<BlockDto> = serde_json::from_str(&blocks_json.as_string().unwrap_or_default())
        .map_err(|e| JsValue::from_str(&format!("Invalid blocks JSON: {}", e)))?;

    BLOCKS.with(|store| {
        store.borrow_mut().insert(page_id.clone(), blocks.clone());
    });
    // History stack initialization removed — will be re-added with proper undo/redo

    let state = serde_json::json!({
        "blocks": blocks,
        "canUndo": false,
        "canRedo": false,
        "stateHash": compute_hash(&blocks),
    });
    serde_wasm_bindgen::to_value(&state).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn dispatch(page_id: String, command_js: JsValue) -> Result<JsValue, JsValue> {
    let cmd_json: serde_json::Value =
        serde_json::from_str(&command_js.as_string().unwrap_or_default())
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
    let cmd: OutlinerCommand = serde_json::from_value(cmd_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid command: {}", e)))?;

    let result = BLOCKS.with(|store| {
        let mut store = store.borrow_mut();
        let blocks = store.entry(page_id).or_default();

        match apply_command(blocks, &cmd) {
            Ok(()) => {
                serde_json::json!({ "accepted": true, "stateHash": compute_hash(blocks) })
            }
            Err(e) => {
                serde_json::json!({ "accepted": false, "stateHash": compute_hash(blocks), "error": e })
            }
        }
    });

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn undo(_page_id: String) -> Result<JsValue, JsValue> {
    // Undo/redo disabled for now — will be re-added with proper history support
    let r = serde_json::json!({ "ok": false });
    serde_wasm_bindgen::to_value(&r).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn redo(_page_id: String) -> Result<JsValue, JsValue> {
    // Undo/redo disabled for now — will be re-added with proper history support
    let r = serde_json::json!({ "ok": false });
    serde_wasm_bindgen::to_value(&r).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── HistoryStack bridge (WASM) ────────────────────────────────────
//
// The `HistoryStack` in `outliner/history.rs` is a pure command stack —
// it knows nothing about blocks. To make it useful from React we wrap
// it in `WasmHistoryStack`, which:
//   • owns the current block list (`Vec<BlockDto>`)
//   • applies an incoming `OutlinerCommand` to the blocks and records
//     it in the history stack
//   • on undo, inverts the recorded command and applies the inverse
//   • on redo, re-applies the recorded command
//
// A `thread_local!` registry maps `u32` stack ids to instances.
// The React hook calls `history_new` once per page, `history_free` on
// page change, and `history_apply / history_undo / history_redo` on
// every user action.

/// A `HistoryStack` paired with a live block list.
///
/// `capacity` is the maximum number of undo steps. Commands beyond
/// capacity evict the oldest entry.
pub struct WasmHistoryStack {
    blocks: Vec<BlockDto>,
    history: HistoryStack,
}

impl WasmHistoryStack {
    /// Create a new bridge with the given initial blocks.
    pub fn from_blocks(blocks: Vec<BlockDto>, capacity: usize) -> Self {
        Self {
            blocks,
            history: HistoryStack::new(capacity),
        }
    }

    /// Apply a command: mutate blocks and push onto the history stack.
    pub fn apply(&mut self, cmd: HistoryCommand) -> Result<(), String> {
        apply_history_command(&mut self.blocks, &cmd)?;
        self.history.push(cmd);
        Ok(())
    }

    /// Undo the last command by inverting it and re-applying.
    /// Returns `true` if there was something to undo.
    pub fn undo(&mut self) -> bool {
        let Some(cmd) = self.history.undo() else {
            return false;
        };
        let inverse = history::invert_command(&cmd);
        // The inverse must succeed — if it doesn't, restore by re-applying original.
        if apply_history_command(&mut self.blocks, &inverse).is_err() {
            // Best-effort: re-apply original to keep blocks consistent.
            let _ = apply_history_command(&mut self.blocks, &cmd);
            return false;
        }
        true
    }

    /// Redo the next command.
    /// Returns `true` if there was something to redo.
    pub fn redo(&mut self) -> bool {
        let Some(cmd) = self.history.redo() else {
            return false;
        };
        if apply_history_command(&mut self.blocks, &cmd).is_err() {
            return false;
        }
        true
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn current_blocks(&self) -> &[BlockDto] {
        &self.blocks
    }
}

thread_local! {
    /// Registry of `WasmHistoryStack` instances, indexed by `u32` id.
    /// Reused slots from freed stacks are filled before appending.
    static HISTORY_STACKS: RefCell<Vec<Option<WasmHistoryStack>>> = RefCell::new(Vec::new());
}

/// Take a free slot in the registry: reuse a freed slot, or push a new one.
fn alloc_stack_id(stack: WasmHistoryStack) -> u32 {
    HISTORY_STACKS.with(|reg| {
        let mut reg = reg.borrow_mut();
        if let Some((idx, slot)) = reg.iter_mut().enumerate().find(|(_, s)| s.is_none()) {
            *slot = Some(stack);
            idx as u32
        } else {
            reg.push(Some(stack));
            (reg.len() - 1) as u32
        }
    })
}

/// Initialize a `WasmHistoryStack` from a JSON array of blocks.
/// Returns the stack id (u32) to be used for subsequent calls.
#[wasm_bindgen]
pub fn history_new(blocks_js: JsValue) -> Result<u32, JsValue> {
    let blocks: Vec<BlockDto> = serde_wasm_bindgen::from_value(blocks_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid blocks: {e}")))?;
    let stack = WasmHistoryStack::from_blocks(blocks, 100);
    Ok(alloc_stack_id(stack))
}

/// Push a command onto the history stack and apply it to the block list.
/// Returns the new block list as JSON. Returns `null` if the stack id is
/// invalid.
#[wasm_bindgen]
pub fn history_apply(stack_id: u32, command_js: JsValue) -> Result<JsValue, JsValue> {
    let cmd: HistoryCommand = serde_wasm_bindgen::from_value(command_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid command: {e}")))?;
    HISTORY_STACKS.with(|reg| {
        let mut reg = reg.borrow_mut();
        let slot = reg
            .get_mut(stack_id as usize)
            .ok_or_else(|| JsValue::from_str("Stack not found"))?;
        let stack = slot
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Stack was freed"))?;
        stack.apply(cmd).map_err(|e| JsValue::from_str(&e))?;
        serde_wasm_bindgen::to_value(stack.current_blocks())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    })
}

/// Undo the last command. Returns the new block list as JSON, or `null`
/// if there is nothing to undo (or the stack id is invalid).
#[wasm_bindgen]
pub fn history_undo(stack_id: u32) -> Result<JsValue, JsValue> {
    HISTORY_STACKS.with(|reg| {
        let mut reg = reg.borrow_mut();
        let slot = match reg.get_mut(stack_id as usize) {
            Some(s) => s,
            None => return Ok(JsValue::NULL),
        };
        let stack = match slot.as_mut() {
            Some(s) => s,
            None => return Ok(JsValue::NULL),
        };
        if !stack.undo() {
            return Ok(JsValue::NULL);
        }
        serde_wasm_bindgen::to_value(stack.current_blocks())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    })
}

/// Redo the next command. Returns the new block list as JSON, or `null`
/// if there is nothing to redo (or the stack id is invalid).
#[wasm_bindgen]
pub fn history_redo(stack_id: u32) -> Result<JsValue, JsValue> {
    HISTORY_STACKS.with(|reg| {
        let mut reg = reg.borrow_mut();
        let slot = match reg.get_mut(stack_id as usize) {
            Some(s) => s,
            None => return Ok(JsValue::NULL),
        };
        let stack = match slot.as_mut() {
            Some(s) => s,
            None => return Ok(JsValue::NULL),
        };
        if !stack.redo() {
            return Ok(JsValue::NULL);
        }
        serde_wasm_bindgen::to_value(stack.current_blocks())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    })
}

/// Returns `true` if the stack has at least one command to undo.
#[wasm_bindgen]
pub fn history_can_undo(stack_id: u32) -> bool {
    HISTORY_STACKS.with(|reg| {
        let reg = reg.borrow();
        reg.get(stack_id as usize)
            .and_then(|s| s.as_ref())
            .map(|s| s.can_undo())
            .unwrap_or(false)
    })
}

/// Returns `true` if the stack has at least one command to redo.
#[wasm_bindgen]
pub fn history_can_redo(stack_id: u32) -> bool {
    HISTORY_STACKS.with(|reg| {
        let reg = reg.borrow();
        reg.get(stack_id as usize)
            .and_then(|s| s.as_ref())
            .map(|s| s.can_redo())
            .unwrap_or(false)
    })
}

/// Free a `WasmHistoryStack`. The id may be reused by a future
/// `history_new` call.
#[wasm_bindgen]
pub fn history_free(stack_id: u32) {
    HISTORY_STACKS.with(|reg| {
        let mut reg = reg.borrow_mut();
        if let Some(slot) = reg.get_mut(stack_id as usize) {
            *slot = None;
        }
    });
}

/// Apply a `history::OutlinerCommand` to a mutable block list.
///
/// Content commands (SetContent, AutocompleteInsert) are applied via
/// direct field updates. All other variants delegate to
/// `apply_structural_mutation` from `outliner/tree`.
fn apply_history_command(blocks: &mut Vec<BlockDto>, cmd: &HistoryCommand) -> Result<(), String> {
    match cmd {
        HistoryCommand::SetContent {
            block_id, after, ..
        }
        | HistoryCommand::AutocompleteInsert {
            block_id, after, ..
        } => {
            let block = blocks
                .iter_mut()
                .find(|b| b.id == *block_id)
                .ok_or_else(|| format!("Block not found: {block_id}"))?;
            block.content = after.clone();
            block.updated_at = chrono::Utc::now().to_rfc3339();
            Ok(())
        }
        _ => {
            if tree::apply_structural_mutation(blocks, cmd) {
                Ok(())
            } else {
                Err(format!("Structural mutation failed for command: {cmd}"))
            }
        }
    }
}

#[wasm_bindgen]
pub fn parse_inline(content: String) -> Result<JsValue, JsValue> {
    let parser = InlineParser::new();
    let result = parser.parse(&content);
    let json = serde_json::json!({
        "rawText": result.raw_text,
        "segments": result.segments,
    });
    let json_str = serde_json::to_string(&json)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn ping() -> bool {
    true
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Force Simulation ──────────────────────────────────────────────

#[wasm_bindgen]
pub fn run_force_simulation(
    nodes_json: String,
    edges_json: String,
    params_json: String,
) -> Result<JsValue, JsValue> {
    let nodes: Vec<crate::graph::force_simulation::SimNode> = serde_json::from_str(&nodes_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid nodes JSON: {}", e)))?;
    let edges: Vec<crate::graph::force_simulation::SimEdge> = serde_json::from_str(&edges_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid edges JSON: {}", e)))?;
    let params: crate::graph::force_simulation::SimulationParams =
        serde_json::from_str(&params_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid params JSON: {}", e)))?;

    let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let names: Vec<String> = nodes.iter().map(|n| n.name.clone()).collect();
    let journals: Vec<bool> = nodes.iter().map(|n| n.journal).collect();
    let sources: Vec<usize> = edges.iter().map(|e| e.source_idx).collect();
    let targets: Vec<usize> = edges.iter().map(|e| e.target_idx).collect();

    let mut sim = crate::graph::force_simulation::ForceSimulation::with_params(
        ids, names, journals, sources, targets, params,
    );
    let _result = sim.run();

    let json_str = serde_json::to_string(&sim.nodes())
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

// ── Graph analysis WASM exports ──────────────────────────────────

#[wasm_bindgen]
pub fn graph_detect_clusters(adjacency_json: String, min_size: usize) -> Result<JsValue, JsValue> {
    let adjacency: HashMap<String, Vec<String>> = serde_json::from_str(&adjacency_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid adjacency JSON: {}", e)))?;
    let clusters = crate::graph::analysis::detect_clusters(&adjacency, min_size);
    let json = serde_json::to_string(&clusters)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json))
}

#[wasm_bindgen]
pub fn graph_detect_gaps(adjacency_json: String, threshold: f64) -> Result<JsValue, JsValue> {
    let adjacency: HashMap<String, Vec<String>> = serde_json::from_str(&adjacency_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid adjacency JSON: {}", e)))?;
    let gaps = crate::graph::analysis::detect_gaps(&adjacency, threshold);
    let json = serde_json::to_string(&gaps)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json))
}

#[wasm_bindgen]
pub fn graph_pagerank(
    adjacency_json: String,
    iterations: usize,
    damping: f64,
) -> Result<JsValue, JsValue> {
    let adjacency: HashMap<String, Vec<String>> = serde_json::from_str(&adjacency_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid adjacency JSON: {}", e)))?;
    let scores = crate::graph::analysis::compute_pagerank(&adjacency, iterations, damping);
    let json = serde_json::to_string(&scores)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json))
}

#[wasm_bindgen]
pub fn graph_density(node_count: usize, edge_count: usize) -> f64 {
    crate::graph::analysis::compute_density(node_count, edge_count)
}

#[wasm_bindgen]
pub fn graph_frontiers(adjacency_json: String) -> Result<JsValue, JsValue> {
    let adjacency: HashMap<String, Vec<String>> = serde_json::from_str(&adjacency_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid adjacency JSON: {}", e)))?;
    let frontiers = crate::graph::analysis::find_frontiers(&adjacency);
    let json = serde_json::to_string(&frontiers)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json))
}

// ── FTS5 Search WASM exports ──────────────────────────────────────

/// Sanitize a user query string for safe use in FTS5 MATCH expressions.
///
/// Returns the legacy single-string form (terms joined with spaces).
/// The new safe behavior strips FTS5 boolean operator words (AND, OR,
/// NOT, NEAR) from the input so they cannot leak into the MATCH
/// expression.
///
/// JS example:
/// ```text
/// const safe = quiltWasm.fts_sanitize('foo AND bar');
/// // → `"foo" "bar"`
/// ```
#[wasm_bindgen]
pub fn fts_sanitize(query: String) -> String {
    crate::search::fts::sanitize_fts5_query(&query)
}

/// Build a complete FTS5 MATCH expression from user input.
///
/// Returns the safe, AND-joined (implicit, via whitespace) MATCH
/// expression, or an empty string if the input produces no FTS5 tokens.
///
/// JS example:
/// ```text
/// const matchExpr = quiltWasm.fts_build_match('hello world');
/// // → `"hello" "world"`
/// const empty = quiltWasm.fts_build_match('AND OR');
/// // → ``
/// ```
#[wasm_bindgen]
pub fn fts_build_match(query: String) -> String {
    crate::search::fts::build_fts5_match_query(&query).unwrap_or_default()
}

/// Build a fuzzy prefix-match query from user input by appending `*`
/// to each alphanumeric term. Strips non-alphanumeric characters.
#[wasm_bindgen]
pub fn fts_fuzzy_query(query: String) -> String {
    crate::search::fts::build_fuzzy_query(&query)
}

/// Generate a search-result snippet with highlight markers.
/// Truncates content longer than `max_length` with an ellipsis.
#[wasm_bindgen]
pub fn fts_snippet(content: String, query: String, max_length: usize) -> String {
    crate::search::fts::generate_snippet(&content, &query, max_length)
}

// ── Projection WASM exports ────────────────────────────────────────
//
// The `WasmProjectionResolver` (in `crates/quilt-core/src/projection/`)
// is a pure-function port of the server's `ProjectionResolver` (slice
// #4). It accepts a `BlockDto` JSON, runs the V1 contract resolution
// algorithm, and returns a `WasmProjectionView` JSON with the same
// shape as the server's `ProjectionView` (plus three WASM-specific
// metadata fields: `wasm_source`, `wasm_contract_id`,
// `wasm_had_conflict`).
//
// The TypeScript bridge (`quilt-ui/src/core/wasm-bridge/wasm-loader.ts`)
// wraps this export with `wasmProjectionResolve(block)` and handles
// the `BlockProperty[]` → `BlockDto` JSON conversion.

/// Resolve the projection view for a single block via the V1 registry.
///
/// `block_json` is a JSON-serialized `BlockDto`. Returns a
/// JSON-serialized `WasmProjectionView` on success. On parse
/// error, returns `Err(JsValue)` with a clear message. The
/// resolver catches all internal errors and returns a default view
/// (the caller never has to handle a "no view" case).
#[wasm_bindgen]
pub fn projection_resolve(block_json: String) -> Result<JsValue, JsValue> {
    let block: BlockDto = serde_json::from_str(&block_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid BlockDto JSON: {e}")))?;
    let view = crate::projection::WasmProjectionResolver::v1().resolve(&block);
    let json = serde_json::to_string(&view)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json))
}

// ── Schema Validation WASM exports ──────────────────────────────────

use crate::schema::classes::{self as schema_classes, Class};
use crate::schema::properties::{self as schema_props, PropertyDefinition};

/// Validate a property value against a property definition.
///
/// Both inputs are JSON strings:
/// - `def_json`: PropertyDefinition as JSON
/// - `value_json`: value to validate as JSON
///
/// Returns `{ valid: true }` or `{ valid: false, error: "..." }`.
#[wasm_bindgen]
pub fn schema_validate_property(def_json: String, value_json: String) -> Result<JsValue, JsValue> {
    let def: PropertyDefinition = serde_json::from_str(&def_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid PropertyDefinition JSON: {e}")))?;
    let value: serde_json::Value = serde_json::from_str(&value_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid value JSON: {e}")))?;

    let result = match schema_props::validate_property(&def, &value) {
        Ok(()) => serde_json::json!({ "valid": true }),
        Err(e) => serde_json::json!({ "valid": false, "error": e }),
    };
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json_str))
}

/// Validate properties against a class definition.
///
/// Inputs are JSON strings:
/// - `class_json`: Class as JSON
/// - `properties_json`: JSON object `{ key: value, ... }`
///
/// Returns `{ valid: true }` or `{ valid: false, errors: [...] }`.
#[wasm_bindgen]
pub fn schema_validate_class(
    class_json: String,
    properties_json: String,
) -> Result<JsValue, JsValue> {
    let class: Class = serde_json::from_str(&class_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid Class JSON: {e}")))?;
    let props: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&properties_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid properties JSON: {e}")))?;

    let result = match schema_classes::validate_class_required_properties(&class, &props) {
        Ok(()) => serde_json::json!({ "valid": true }),
        Err(errors) => serde_json::json!({ "valid": false, "errors": errors }),
    };
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json_str))
}

// ── Strategy Selector WASM exports ────────────────────────────────
//
// The `DefaultStrategySelector` (and the `StrategySelector` trait it
// implements) is the per-block dispatcher the React outliner calls to
// pick a rendering / editing strategy for a Block. We expose a thin
// `#[wasm_bindgen]` wrapper so the front-end can call the selector
// through the same channel as every other Quilt primitive.
//
// The WASM surface is intentionally minimal: a constructor
// (`with_builtins`), a `select(block_json)` call that returns the
// strategy name (or `None`), and an `all_strategies()` accessor. The
// shape of the input JSON is `{ "properties": { "type": "task", ... } }`
// — see `strategy::Block` for the full contract.

use crate::strategy::{DefaultStrategySelector, StrategySelector, select_strategy_from_json};

/// WASM-exposed wrapper around `DefaultStrategySelector`.
///
/// Constructed via `new` (no strategies — purely a base) or
/// `with_builtins` (the canonical task/query/view/agent-run/default
/// registry). Stored as `Box<dyn StrategySelector>` so the future
/// portfolio selector (ADR-0006) can be substituted without API churn.
#[wasm_bindgen]
pub struct WasmStrategySelector {
    inner: Box<dyn StrategySelector>,
}

#[wasm_bindgen]
impl WasmStrategySelector {
    /// Build a selector with the built-in strategies
    /// (task → query → view → agent-run → default).
    #[wasm_bindgen(constructor)]
    pub fn with_builtins() -> WasmStrategySelector {
        WasmStrategySelector {
            inner: Box::new(DefaultStrategySelector::with_builtins()),
        }
    }

    /// Pick a strategy for the given block.
    ///
    /// `block_json` must be a JSON string of the shape
    /// `{"properties":{"type":"task",...}}`. Returns the strategy
    /// name as a string, or `None` if no registered strategy handles
    /// the block (which only happens for empty registries).
    #[wasm_bindgen]
    pub fn select(&self, block_json: &str) -> Option<String> {
        // Delegate the testable JSON-parsing + dispatch logic to
        // `select_strategy_from_json` in `crate::strategy` so the
        // contract is unit-tested on the native target. The WASM
        // shim here is a 1-liner.
        select_strategy_from_json(block_json)
    }

    /// Names of all registered strategies, in registration order.
    ///
    /// Exposed to the React side as `all_strategies` to avoid
    /// clashing with the reserved `all` keyword.
    #[wasm_bindgen]
    pub fn all_strategies(&self) -> Vec<String> {
        self.inner.all().into_iter().map(String::from).collect()
    }
}

// ── Internal ──────────────────────────────────────────────────────

fn compute_hash(blocks: &[BlockDto]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    blocks.len().hash(&mut hasher);
    for b in blocks {
        b.id.hash(&mut hasher);
        b.content.hash(&mut hasher);
        b.parent_id.hash(&mut hasher);
        b.order.to_bits().hash(&mut hasher);
    }
    // Returned as a String (not u64) because the 64-bit hash overflows
    // JavaScript's safe integer range (Number.MAX_SAFE_INTEGER = 2^53-1).
    // Serialising the u64 directly would surface as a number that throws
    // "X can't be represented as a JavaScript number" when wasm-bindgen
    // marshals it across the boundary. String form is safe to compare
    // and the JS side never does arithmetic on it.
    hasher.finish().to_string()
}

fn apply_command(blocks: &mut Vec<BlockDto>, cmd: &OutlinerCommand) -> Result<(), String> {
    match cmd {
        OutlinerCommand::SetContent { block_id, content } => blocks
            .iter_mut()
            .find(|b| &b.id == block_id)
            .map(|b| b.content = content.clone())
            .ok_or_else(|| format!("Block not found: {}", block_id)),

        OutlinerCommand::SplitBlock {
            block_id,
            cursor_pos,
        } => tree::split_block(blocks, block_id, *cursor_pos as u32)
            .map(|_| ())
            .map_err(|e| format!("SplitBlock failed: {:?}", e)),

        OutlinerCommand::MergePrev { block_id } => tree::merge_with_prev(blocks, block_id)
            .map_err(|e| format!("MergePrev failed: {:?}", e)),

        OutlinerCommand::MergeNext { block_id } => tree::merge_with_next(blocks, block_id)
            .map(|_| ())
            .map_err(|e| format!("MergeNext failed: {:?}", e)),

        OutlinerCommand::Indent { block_id } => {
            tree::indent(blocks, block_id).map_err(|e| format!("Indent failed: {:?}", e))
        }

        OutlinerCommand::Outdent { block_id } => {
            tree::outdent(blocks, block_id).map_err(|e| format!("Outdent failed: {:?}", e))
        }

        OutlinerCommand::MoveBlock {
            block_id,
            new_parent_id,
            new_order,
        } => blocks
            .iter_mut()
            .find(|b| &b.id == block_id)
            .map(|b| {
                b.parent_id = Some(new_parent_id.clone());
                b.order = *new_order;
            })
            .ok_or_else(|| format!("Block not found: {}", block_id)),

        OutlinerCommand::CycleMarker { block_id } => cycle_marker(blocks, block_id),

        OutlinerCommand::CyclePriority { block_id } => cycle_priority(blocks, block_id),
    }
}

fn cycle_marker(blocks: &mut [BlockDto], block_id: &str) -> Result<(), String> {
    const MARKER_CYCLE: &[Option<&str>] = &[None, Some("Todo"), Some("Done")];
    let block = blocks
        .iter_mut()
        .find(|b| b.id == block_id)
        .ok_or_else(|| format!("Block not found: {}", block_id))?;

    let current = block.marker.as_deref();
    let next_idx = MARKER_CYCLE
        .iter()
        .position(|m| *m == current)
        .map(|i| (i + 1) % MARKER_CYCLE.len())
        .unwrap_or(0);

    block.marker = MARKER_CYCLE[next_idx].map(String::from);
    Ok(())
}

fn cycle_priority(blocks: &mut [BlockDto], block_id: &str) -> Result<(), String> {
    const PRIORITY_CYCLE: &[Option<&str>] = &[None, Some("A"), Some("B"), Some("C")];
    let block = blocks
        .iter_mut()
        .find(|b| b.id == block_id)
        .ok_or_else(|| format!("Block not found: {}", block_id))?;

    let current = block.priority.as_deref();
    let next_idx = PRIORITY_CYCLE
        .iter()
        .position(|p| *p == current)
        .map(|i| (i + 1) % PRIORITY_CYCLE.len())
        .unwrap_or(0);

    block.priority = PRIORITY_CYCLE[next_idx].map(String::from);
    Ok(())
}

// ── CRDT Sync ─────────────────────────────────────────────────────

use crate::sync::crdt::{ConflictStrategy, CrdtSyncEngine, SyncChange};
use uuid::Uuid;

thread_local! {
    static CRDT_ENGINES: std::cell::RefCell<std::collections::HashMap<String, CrdtSyncEngine>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Initialize a new CRDT engine for the given peer.
/// Returns the peer_id (UUID) that was generated.
#[wasm_bindgen]
pub fn crdt_init(peer_id: String) -> Result<JsValue, JsValue> {
    let uuid = Uuid::parse_str(&peer_id)
        .map_err(|e| JsValue::from_str(&format!("Invalid peer_id UUID: {e}")))?;
    let engine = CrdtSyncEngine::with_peer_id(uuid);

    CRDT_ENGINES.with(|store| {
        store.borrow_mut().insert(peer_id.clone(), engine);
    });

    let result = serde_json::json!({ "ok": true, "peer_id": peer_id });
    Ok(JsValue::from_str(
        &serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))?,
    ))
}

/// Apply a local change to the engine identified by peer_id.
/// `change_json` should be a JSON string with fields:
/// entity_id, entity_type, data (base64 or raw bytes encoded as a string).
///
/// Returns the resulting SyncChange as JSON.
#[wasm_bindgen]
pub fn crdt_local_change(
    peer_id: String,
    entity_id: String,
    entity_type: String,
    data: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let eid = Uuid::parse_str(&entity_id)
        .map_err(|e| JsValue::from_str(&format!("Invalid entity_id UUID: {e}")))?;

    let change = CRDT_ENGINES.with(|store| {
        let mut store = store.borrow_mut();
        let engine = store
            .get_mut(&peer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Engine not found for peer: {peer_id}")))?;
        Ok::<SyncChange, JsValue>(engine.apply_local_change(eid, &entity_type, data))
    })?;

    // Wrap in { accepted: true, change: ... } so the JS caller gets a consistent shape
    let json_str = serde_json::to_string(&serde_json::json!({
        "accepted": true,
        "change": change,
    }))
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsValue::from_str(&json_str))
}

/// Apply a remote change to the engine identified by peer_id.
/// `change_json` should be a JSON string matching the SyncChange structure.
///
/// Returns the ConflictResolution as JSON.
#[wasm_bindgen]
pub fn crdt_remote_change(peer_id: String, change_json: String) -> Result<JsValue, JsValue> {
    let change: SyncChange = serde_json::from_str(&change_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid change JSON: {e}")))?;

    let resolution = CRDT_ENGINES.with(|store| {
        let mut store = store.borrow_mut();
        let engine = store
            .get_mut(&peer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Engine not found for peer: {peer_id}")))?;
        Ok::<_, JsValue>(engine.apply_remote_change_with_resolution(&change))
    })?;

    // Return a simplified JSON response
    let result = serde_json::json!({
        "entity_id": resolution.entity_id.to_string(),
        "accepted": resolution.winning_change.as_ref().map(|w| w.peer_id.to_string() == peer_id).unwrap_or(false),
        "has_conflict": resolution.conflict_marker.is_some(),
        "winning_peer": resolution.winning_change.as_ref().map(|w| w.peer_id.to_string()),
    });

    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json_str))
}

/// Get the current state of an engine as JSON.
/// Returns all entities and their version metadata.
#[wasm_bindgen]
pub fn crdt_get_state(peer_id: String) -> Result<JsValue, JsValue> {
    CRDT_ENGINES.with(|store| {
        let store = store.borrow();
        let engine = store
            .get(&peer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Engine not found for peer: {peer_id}")))?;

        let entities: Vec<serde_json::Value> = engine
            .export_full()
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "entity_id": c.entity_id.to_string(),
                    "entity_type": c.entity_type,
                    "data": c.data,
                    "version": c.version,
                    "peer_id": c.peer_id.to_string(),
                    "timestamp": c.timestamp,
                })
            })
            .collect();

        let result = serde_json::json!({
            "peer_id": engine.peer_id().to_string(),
            "current_version": engine.current_version(),
            "entity_count": engine.entity_count(),
            "strategy": match engine.strategy() {
                ConflictStrategy::LastWriteWins => "LastWriteWins",
                ConflictStrategy::PreserveBoth => "PreserveBoth",
                ConflictStrategy::Manual => "Manual",
            },
            "entities": entities,
        });

        let json_str = serde_json::to_string(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
        Ok(JsValue::from_str(&json_str))
    })
}

/// Export the engine state as an array of SyncChange JSON objects,
/// suitable for sending to a server or another peer.
#[wasm_bindgen]
pub fn crdt_export(peer_id: String) -> Result<JsValue, JsValue> {
    let changes = CRDT_ENGINES.with(|store| {
        let store = store.borrow();
        let engine = store
            .get(&peer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Engine not found for peer: {peer_id}")))?;
        Ok::<Vec<SyncChange>, JsValue>(engine.export_full())
    })?;

    let json_str = serde_json::to_string(&changes)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json_str))
}

/// Import a batch of changes into the engine.
/// Returns the number of accepted changes.
#[wasm_bindgen]
pub fn crdt_import_batch(peer_id: String, changes_json: String) -> Result<JsValue, JsValue> {
    let changes: Vec<SyncChange> = serde_json::from_str(&changes_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid changes JSON: {e}")))?;

    let accepted = CRDT_ENGINES.with(|store| {
        let mut store = store.borrow_mut();
        let engine = store
            .get_mut(&peer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Engine not found for peer: {peer_id}")))?;
        Ok::<usize, JsValue>(engine.import_batch(&changes))
    })?;

    let result = serde_json::json!({ "accepted": accepted });
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {e}")))?;
    Ok(JsValue::from_str(&json_str))
}

// ── Connection Scoring ─────────────────────────────────────────────

#[wasm_bindgen]
pub fn scoring_jaccard(set_a_json: String, set_b_json: String) -> Result<JsValue, JsValue> {
    let set_a: Vec<String> = serde_json::from_str(&set_a_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid set_a JSON: {e}")))?;
    let set_b: Vec<String> = serde_json::from_str(&set_b_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid set_b JSON: {e}")))?;
    let result = crate::scoring::connection::jaccard_similarity(&set_a, &set_b);
    Ok(JsValue::from_f64(result))
}

#[wasm_bindgen]
pub fn scoring_temporal(ts_a: i64, ts_b: i64, halflife: f64) -> f64 {
    crate::scoring::connection::temporal_decay(ts_a, ts_b, halflife)
}

#[wasm_bindgen]
pub fn scoring_composite(structural: f64, temporal: f64, w_struct: f64, w_temporal: f64) -> f64 {
    crate::scoring::connection::composite_score(structural, temporal, w_struct, w_temporal)
}

// ── Query DSL — WASM exports ─────────────────────────────────────

/// Parse a query DSL string and return the AST as a JSON string.
///
/// On success returns a JSON object: `{ "ok": true, "ast": <QueryExpr> }`.
/// On failure returns a JSON object: `{ "ok": false, "error": "..." }`.
#[wasm_bindgen]
pub fn query_parse(query: String) -> Result<JsValue, JsValue> {
    use crate::query::QueryParser;

    let parser = QueryParser;
    match parser.parse(&query) {
        Ok(expr) => {
            let json_str = serde_json::to_string(&serde_json::json!({
                "ok": true,
                "ast": expr,
            }))
            .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
            Ok(JsValue::from_str(&json_str))
        }
        Err(e) => {
            let json_str = serde_json::to_string(&serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            }))
            .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
            Ok(JsValue::from_str(&json_str))
        }
    }
}

/// Parse and validate a query DSL string.
///
/// Returns a JSON object:
/// ```json
/// { "valid": true, "error": null, "ast": <QueryExpr> }
/// ```
/// or
/// ```json
/// { "valid": false, "error": "error message", "ast": null }
/// ```
#[wasm_bindgen]
pub fn query_validate(query: String) -> Result<JsValue, JsValue> {
    use crate::query::QueryParser;

    let parser = QueryParser;
    match parser.parse(&query) {
        Ok(expr) => {
            let result = serde_json::json!({
                "valid": true,
                "error": null,
                "ast": expr,
            });
            let json_str = serde_json::to_string(&result)
                .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
            Ok(JsValue::from_str(&json_str))
        }
        Err(e) => {
            let result = serde_json::json!({
                "valid": false,
                "error": e.to_string(),
                "ast": null,
            });
            let json_str = serde_json::to_string(&result)
                .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
            Ok(JsValue::from_str(&json_str))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockDto;

    fn make_block(
        id: &str,
        parent_id: Option<&str>,
        content: &str,
        level: u8,
        order: f64,
    ) -> BlockDto {
        BlockDto {
            id: id.to_string(),
            page_id: "page1".to_string(),
            parent_id: parent_id.map(String::from),
            content: content.to_string(),
            order,
            level,
            marker: None,
            priority: None,
            collapsed: false,
            properties: serde_json::json!({}),
            refs: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            created_by: None,
        }
    }

    #[test]
    fn test_apply_set_content() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let cmd = OutlinerCommand::SetContent {
            block_id: "b1".to_string(),
            content: "World".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks[0].content, "World");
    }

    #[test]
    fn test_apply_set_content_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let cmd = OutlinerCommand::SetContent {
            block_id: "bogus".to_string(),
            content: "World".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_apply_split_block() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".to_string(),
            cursor_pos: 5,
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "Hello");
        assert_eq!(blocks[1].content, " World");
    }

    #[test]
    fn test_apply_split_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let cmd = OutlinerCommand::SplitBlock {
            block_id: "bogus".to_string(),
            cursor_pos: 3,
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_merge_prev() {
        let mut blocks = vec![
            make_block("b1", None, "Hello", 1, 1.0),
            make_block("b2", None, " World", 1, 2.0),
        ];
        let cmd = OutlinerCommand::MergePrev {
            block_id: "b2".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Hello World");
    }

    #[test]
    fn test_apply_merge_prev_no_previous() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let cmd = OutlinerCommand::MergePrev {
            block_id: "b1".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_merge_next() {
        let mut blocks = vec![
            make_block("b1", None, "Hello", 1, 1.0),
            make_block("b2", None, " World", 1, 2.0),
        ];
        let cmd = OutlinerCommand::MergeNext {
            block_id: "b1".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Hello World");
    }

    #[test]
    fn test_apply_merge_next_no_next() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let cmd = OutlinerCommand::MergeNext {
            block_id: "b1".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_indent() {
        let mut blocks = vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", None, "Block 2", 1, 2.0),
        ];
        let cmd = OutlinerCommand::Indent {
            block_id: "b2".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks[1].parent_id, Some("b1".to_string()));
        assert_eq!(blocks[1].level, 2);
    }

    #[test]
    fn test_apply_indent_no_previous_sibling() {
        let mut blocks = vec![make_block("b1", None, "Block 1", 1, 1.0)];
        let cmd = OutlinerCommand::Indent {
            block_id: "b1".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_outdent() {
        let mut blocks = vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", Some("b1"), "Block 2", 2, 1.5),
        ];
        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks[1].parent_id, None);
        assert_eq!(blocks[1].level, 1);
    }

    #[test]
    fn test_apply_outdent_root_block() {
        let mut blocks = vec![make_block("b1", None, "Block 1", 1, 1.0)];
        let cmd = OutlinerCommand::Outdent {
            block_id: "b1".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_move_block() {
        let mut blocks = vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", None, "Block 2", 1, 2.0),
        ];
        let cmd = OutlinerCommand::MoveBlock {
            block_id: "b2".to_string(),
            new_parent_id: "b1".to_string(),
            new_order: 1.5,
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_ok());
        assert_eq!(blocks[1].parent_id, Some("b1".to_string()));
        assert!((blocks[1].order - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_apply_move_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "Block 1", 1, 1.0)];
        let cmd = OutlinerCommand::MoveBlock {
            block_id: "bogus".to_string(),
            new_parent_id: "b1".to_string(),
            new_order: 1.5,
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_cycle_marker_cycles_through_values() {
        let mut blocks = vec![make_block("b1", None, "Task", 1, 1.0)];
        let cmd = OutlinerCommand::CycleMarker {
            block_id: "b1".to_string(),
        };

        assert_eq!(blocks[0].marker, None);

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].marker, Some("Todo".to_string()));

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].marker, Some("Done".to_string()));

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].marker, None);
    }

    #[test]
    fn test_apply_cycle_marker_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "Task", 1, 1.0)];
        let cmd = OutlinerCommand::CycleMarker {
            block_id: "bogus".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_cycle_priority_cycles_through_values() {
        let mut blocks = vec![make_block("b1", None, "Task", 1, 1.0)];
        let cmd = OutlinerCommand::CyclePriority {
            block_id: "b1".to_string(),
        };

        assert_eq!(blocks[0].priority, None);

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].priority, Some("A".to_string()));

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].priority, Some("B".to_string()));

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].priority, Some("C".to_string()));

        apply_command(&mut blocks, &cmd).unwrap();
        assert_eq!(blocks[0].priority, None);
    }

    #[test]
    fn test_apply_cycle_priority_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "Task", 1, 1.0)];
        let cmd = OutlinerCommand::CyclePriority {
            block_id: "bogus".to_string(),
        };
        let result = apply_command(&mut blocks, &cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_set_content_updates_hashable_state() {
        let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
        let hash_before = compute_hash(&blocks);
        let cmd = OutlinerCommand::SetContent {
            block_id: "b1".to_string(),
            content: "Changed".to_string(),
        };
        apply_command(&mut blocks, &cmd).unwrap();
        let hash_after = compute_hash(&blocks);
        assert_ne!(
            hash_before, hash_after,
            "hash must change after content update"
        );
    }

    #[test]
    fn test_apply_indent_updates_hashable_state() {
        let mut blocks = vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ];
        let hash_before = compute_hash(&blocks);
        let cmd = OutlinerCommand::Indent {
            block_id: "b2".to_string(),
        };
        apply_command(&mut blocks, &cmd).unwrap();
        let hash_after = compute_hash(&blocks);
        assert_ne!(hash_before, hash_after, "hash must change after indent");
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 11 — WasmHistoryStack bridge (outliner/history ↔ WASM)
    // ═══════════════════════════════════════════════════════════════

    use crate::outliner::history::OutlinerCommand as H;

    fn make_block(
        id: &str,
        parent_id: Option<&str>,
        content: &str,
        level: u8,
        order: f64,
    ) -> BlockDto {
        BlockDto {
            id: id.to_string(),
            page_id: "page1".to_string(),
            parent_id: parent_id.map(String::from),
            content: content.to_string(),
            order,
            level,
            marker: None,
            priority: None,
            collapsed: false,
            properties: serde_json::json!({}),
            refs: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            created_by: None,
        }
    }

    fn stack_with(blocks: Vec<BlockDto>) -> WasmHistoryStack {
        WasmHistoryStack::from_blocks(blocks, 100)
    }

    /// Apply a command to a `WasmHistoryStack` and unwrap, for test brevity.
    macro_rules! apply_ok {
        ($stack:expr, $cmd:expr) => {
            $stack.apply($cmd).expect("apply should succeed")
        };
    }

    // ── Empty stack ──

    #[test]
    fn wasm_history_stack_starts_empty() {
        let stack = stack_with(vec![make_block("b1", None, "Hello", 1, 1.0)]);
        assert!(!stack.can_undo(), "fresh stack has nothing to undo");
        assert!(!stack.can_redo(), "fresh stack has nothing to redo");
        assert_eq!(stack.current_blocks().len(), 1);
    }

    #[test]
    fn wasm_history_stack_undo_empty_returns_false() {
        let mut stack = stack_with(vec![make_block("b1", None, "Hi", 1, 1.0)]);
        assert!(!stack.undo(), "undo on fresh stack returns false");
        assert!(!stack.redo(), "redo on fresh stack returns false");
    }

    // ── Apply + undo + redo for content changes ──

    #[test]
    fn wasm_history_apply_records_content_command() {
        let mut stack = stack_with(vec![make_block("b1", None, "First", 1, 1.0)]);
        let cmd = H::SetContent {
            block_id: "b1".into(),
            before: "First".into(),
            after: "Updated".into(),
        };
        apply_ok!(stack, cmd);
        assert_eq!(stack.current_blocks()[0].content, "Updated");
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn wasm_history_undo_reverts_content_change() {
        let mut stack = stack_with(vec![make_block("b1", None, "First", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "First".into(),
                after: "Updated".into(),
            }
        );
        assert!(stack.undo(), "undo should succeed");
        assert_eq!(stack.current_blocks()[0].content, "First");
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    #[test]
    fn wasm_history_redo_replays_content_change() {
        let mut stack = stack_with(vec![make_block("b1", None, "First", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "First".into(),
                after: "Updated".into(),
            }
        );
        assert!(stack.undo());
        assert!(stack.redo(), "redo should succeed");
        assert_eq!(stack.current_blocks()[0].content, "Updated");
    }

    // ── Apply + undo + redo for structural changes ──

    #[test]
    fn wasm_history_apply_records_split() {
        let mut stack = stack_with(vec![make_block("b1", None, "Hello World", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SplitBlock {
                block_id: "b1".into(),
                new_block_id: "b2".into(),
                first_part: "Hello".into(),
                second_part: " World".into(),
            }
        );
        assert_eq!(stack.current_blocks().len(), 2);
        assert_eq!(stack.current_blocks()[0].content, "Hello");
        assert_eq!(stack.current_blocks()[1].content, " World");
    }

    #[test]
    fn wasm_history_undo_split_via_inverse_merge() {
        let mut stack = stack_with(vec![make_block("b1", None, "Hello World", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SplitBlock {
                block_id: "b1".into(),
                new_block_id: "b2".into(),
                first_part: "Hello".into(),
                second_part: " World".into(),
            }
        );
        assert!(stack.undo());
        assert_eq!(stack.current_blocks().len(), 1);
        assert_eq!(stack.current_blocks()[0].content, "Hello World");
    }

    #[test]
    fn wasm_history_apply_records_indent() {
        let mut stack = stack_with(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]);
        apply_ok!(
            stack,
            H::Indent {
                block_id: "b2".into(),
                old_parent: None,
                old_order: 2.0,
                new_parent: Some("b1".into()),
                new_order: 2.001,
            }
        );
        assert_eq!(stack.current_blocks()[1].parent_id.as_deref(), Some("b1"));
    }

    #[test]
    fn wasm_history_undo_indent_restores_root() {
        let mut stack = stack_with(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]);
        apply_ok!(
            stack,
            H::Indent {
                block_id: "b2".into(),
                old_parent: None,
                old_order: 2.0,
                new_parent: Some("b1".into()),
                new_order: 2.001,
            }
        );
        assert!(stack.undo());
        assert!(stack.current_blocks()[1].parent_id.is_none());
    }

    // ── Redo-buffer truncation ──

    #[test]
    fn wasm_history_new_command_after_undo_clears_redo() {
        let mut stack = stack_with(vec![make_block("b1", None, "First", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "First".into(),
                after: "Updated".into(),
            }
        );
        assert!(stack.undo());
        assert!(stack.can_redo());
        // A new apply must truncate the redo buffer.
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "First".into(),
                after: "Different".into(),
            }
        );
        assert!(
            !stack.can_redo(),
            "redo buffer must be cleared after new apply"
        );
    }

    #[test]
    fn wasm_history_multiple_undos_in_reverse() {
        let mut stack = stack_with(vec![make_block("b1", None, "A", 1, 1.0)]);
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "A".into(),
                after: "B".into(),
            }
        );
        apply_ok!(
            stack,
            H::SetContent {
                block_id: "b1".into(),
                before: "B".into(),
                after: "C".into(),
            }
        );
        assert!(stack.undo());
        assert_eq!(stack.current_blocks()[0].content, "B");
        assert!(stack.undo());
        assert_eq!(stack.current_blocks()[0].content, "A");
    }

    // ── AutocompleteInsert uses the same content path ──

    #[test]
    fn wasm_history_autocomplete_insert_undo_redo() {
        let mut stack = stack_with(vec![make_block("b1", None, "see [[", 1, 1.0)]);
        apply_ok!(
            stack,
            H::AutocompleteInsert {
                block_id: "b1".into(),
                before: "see [[".into(),
                after: "see [[Project]]".into(),
                trigger: "page".into(),
            }
        );
        assert_eq!(stack.current_blocks()[0].content, "see [[Project]]");
        assert!(stack.undo());
        assert_eq!(stack.current_blocks()[0].content, "see [[");
        assert!(stack.redo());
        assert_eq!(stack.current_blocks()[0].content, "see [[Project]]");
    }

    // ── apply_history_command error paths ──

    #[test]
    fn wasm_history_apply_missing_block_fails() {
        let mut stack = stack_with(vec![make_block("b1", None, "Hi", 1, 1.0)]);
        let result = stack.apply(H::SetContent {
            block_id: "bogus".into(),
            before: "Hi".into(),
            after: "Hello".into(),
        });
        assert!(result.is_err(), "missing block must error");
        // History must NOT be poisoned — state is unchanged.
        assert!(!stack.can_undo());
    }

    #[test]
    fn wasm_history_apply_missing_block_does_not_push_to_history() {
        // If apply fails, the history stack must remain consistent.
        let mut stack = stack_with(vec![make_block("b1", None, "Hi", 1, 1.0)]);
        let ok = stack.apply(H::SetContent {
            block_id: "bogus".into(),
            before: "Hi".into(),
            after: "Hello".into(),
        });
        assert!(ok.is_err());
        assert!(!stack.can_undo(), "no command was pushed");
    }

    // ── JSON round-trip (the WASM boundary format) ──

    #[test]
    fn wasm_history_outliner_command_serde_uses_tagged_format() {
        let cmd = H::SetContent {
            block_id: "b1".into(),
            before: "old".into(),
            after: "new".into(),
        };
        let json = serde_json::to_value(&cmd).expect("serialize");
        // Tagged format: { "type": "setContent", "blockId": "b1", ... }
        assert_eq!(json["type"], "setContent");
        assert_eq!(json["blockId"], "b1");
        assert_eq!(json["before"], "old");
        assert_eq!(json["after"], "new");
    }

    #[test]
    fn wasm_history_outliner_command_deserializes_from_tagged_json() {
        let json = serde_json::json!({
            "type": "splitBlock",
            "blockId": "b1",
            "newBlockId": "b2",
            "firstPart": "Hello",
            "secondPart": " World"
        });
        let cmd: H = serde_json::from_value(json).expect("deserialize");
        match cmd {
            H::SplitBlock {
                block_id,
                new_block_id,
                first_part,
                second_part,
            } => {
                assert_eq!(block_id, "b1");
                assert_eq!(new_block_id, "b2");
                assert_eq!(first_part, "Hello");
                assert_eq!(second_part, " World");
            }
            other => panic!("Expected SplitBlock, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 12 — StrategySelector WASM bridge
    // ═══════════════════════════════════════════════════════════════
    //
    // The `StrategySelector` in `crate::strategy` decides which rendering /
    // editing strategy a Block should use. The outliner calls it on every
    // block. These tests pin the WASM contract:
    //   • `with_builtins()` exposes the canonical registry.
    //   • `select(block_json)` returns the strategy name as a string.
    //   • `all_strategies()` returns the registered names in order.
    //   • All entry points are Send+Sync (compile-time, no test) and
    //     safe to call from any WASM thread.
    //
    // The actual `Block::from_json` / JSON-contract tests live in
    // `crate::strategy` (and run on the native target) — the WASM
    // bridge itself is a 3-line `select_strategy_from_json` call.
    // We re-test the contract here for completeness.

    use crate::strategy::{Block, DefaultStrategySelector, StrategySelector};

    #[test]
    fn wasm_default_strategy_selector_selects_task_for_type_task() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"type":"task"}}"#).unwrap();
        let picked = sel.select(&block).expect("must pick a strategy");
        assert_eq!(picked.name(), "task");
    }

    #[test]
    fn wasm_default_strategy_selector_selects_query_for_type_query() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"type":"query"}}"#).unwrap();
        let picked = sel.select(&block).expect("must pick a strategy");
        assert_eq!(picked.name(), "query");
    }

    #[test]
    fn wasm_default_strategy_selector_selects_view_for_type_view() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"type":"view"}}"#).unwrap();
        let picked = sel.select(&block).expect("must pick a strategy");
        assert_eq!(picked.name(), "view");
    }

    #[test]
    fn wasm_default_strategy_selector_selects_agent_run_for_type_agent_run() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"type":"agent-run"}}"#).unwrap();
        let picked = sel.select(&block).expect("must pick a strategy");
        assert_eq!(picked.name(), "agent-run");
    }

    #[test]
    fn wasm_default_strategy_selector_falls_back_to_default_for_unknown_type() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"type":"something-else"}}"#).unwrap();
        let picked = sel.select(&block).expect("default always matches");
        assert_eq!(picked.name(), "default");
    }

    #[test]
    fn wasm_default_strategy_selector_falls_back_to_default_for_block_without_properties() {
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json("{}").unwrap();
        let picked = sel.select(&block).expect("default matches empty blocks");
        assert_eq!(picked.name(), "default");
    }

    #[test]
    fn wasm_default_strategy_selector_all_lists_builtins_in_canonical_order() {
        let sel = DefaultStrategySelector::with_builtins();
        let names = sel.all();
        assert_eq!(
            names,
            vec!["task", "query", "view", "agent-run", "default"],
            "WASM-bound selector must list builtins in the same order as the native one"
        );
    }

    #[test]
    fn wasm_default_strategy_selector_handles_blocks_with_unrelated_properties() {
        // `priority:: A` is metadata, not a strategy role. The selector
        // must not get confused by non-`type` keys.
        let sel = DefaultStrategySelector::with_builtins();
        let block = Block::from_json(r#"{"properties":{"priority":"A"}}"#).unwrap();
        let picked = sel.select(&block).expect("default always matches");
        assert_eq!(picked.name(), "default");
    }

    #[test]
    fn wasm_default_strategy_selector_trait_object_compiles() {
        // Compile-time check: the selector must be usable as
        // `Box<dyn StrategySelector>` for the future PortfolioScorer
        // integration. This is the only ergonomic contract that the
        // WASM bridge relies on (the bridge stores a Box<dyn ...>).
        let sel: Box<dyn StrategySelector> = Box::new(DefaultStrategySelector::with_builtins());
        let block = Block::from_json(r#"{"properties":{"type":"task"}}"#).unwrap();
        assert_eq!(sel.select(&block).unwrap().name(), "task");
    }
}
