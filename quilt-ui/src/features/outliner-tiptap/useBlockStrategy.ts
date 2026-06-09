//! `useBlockStrategy` — React hook that returns the strategy name a
//! `Block` should be rendered / edited with.
//!
//! The strategy decision lives in `quilt-core` (WASM) — `BlockRow`
//! stays free of "what kind of block is this?" branching, and the
//! WASM module is the single source of truth for strategy names
//! (`"task"`, `"query"`, `"view"`, `"agent-run"`, `"default"`).
//!
//! ## Fallback
//!
//! When the WASM module is not loaded (slow network, build not yet
//! emitted, `WasmProvider` reported an error, or the bridge threw),
//! the hook falls back to a **JS-only re-implementation** of the
//! same selector — mirroring the Rust `DefaultStrategySelector` so
//! the UI keeps working while the WASM binary is still on the wire.
//! This matches the spec for road-map #26: "fall back to the
//! existing JS-based logic in BlockRow".
//!
//! As soon as the WASM module finishes loading, the hook re-runs
//! and returns the WASM answer (which should match the JS fallback
//! — they implement the same rules).

import { useMemo } from 'react'
import type { Block } from '@shared/types/api'
import { useWasm } from '@core/wasm-bridge/WasmProvider'
import { wasmStrategySelect } from '@core/wasm-bridge/wasm-loader'

/** All known strategy names. Kept in sync with `quilt-core/strategy.rs`. */
export type BlockStrategyName =
  | 'task'
  | 'query'
  | 'view'
  | 'agent-run'
  | 'default'

/** Fallback strategy when WASM is not available AND we can't infer one. */
const FALLBACK_STRATEGY: BlockStrategyName = 'default'

/**
 * JS-only re-implementation of `DefaultStrategySelector::with_builtins()`.
 *
 * Mirrors `crates/quilt-core/src/strategy.rs` — same registration
 * order (task → query → view → agent-run → default), same `can_handle`
 * rule ("matches the `type::` property"). Used when the WASM module
 * is not available.
 */
export function selectStrategyJs(block: Block | undefined | null): BlockStrategyName {
  if (!block) return FALLBACK_STRATEGY
  const type = block.properties?.find(p => p.key === 'type')?.value
  const typeStr = type == null ? null : String(type)
  switch (typeStr) {
    case 'task':
      return 'task'
    case 'query':
      return 'query'
    case 'view':
      return 'view'
    case 'agent-run':
      return 'agent-run'
    default:
      return FALLBACK_STRATEGY
  }
}

/**
 * Pick a strategy for the given block.
 *
 * Returns a stable string for the same block reference (React's
 * `useMemo` keyed on the block). When the block reference changes,
 * the result is re-computed — which means: re-render BlockRow with a
 * different block → hook returns the new strategy.
 *
 * The hook never throws. WASM errors, missing modules, or an
 * unloaded engine all collapse to the JS fallback selector, which
 * mirrors the Rust behaviour.
 */
export function useBlockStrategy(block: Block): BlockStrategyName {
  const { loaded, error } = useWasm()

  return useMemo<BlockStrategyName>(() => {
    // The block can be `undefined` defensively — keep the hook
    // total so callers don't need to null-check.
    if (!block) return FALLBACK_STRATEGY

    // WASM is not ready → fall back. This is the common path during
    // cold start (the 1.7 MB binary is loaded lazily, so the very
    // first render almost always takes this branch). The JS
    // fallback re-implements the same rules the Rust selector uses,
    // so the UI behaves identically with or without WASM.
    if (!loaded || error) {
      return selectStrategyJs(block)
    }

    try {
      const result = wasmStrategySelect(block)
      if (typeof result === 'string' && result.length > 0) {
        return result as BlockStrategyName
      }
    } catch {
      // Bridge threw (WASM module not built, network blip, etc.).
      // Collapse to the JS fallback — the UI keeps working.
    }
    return selectStrategyJs(block)
  }, [loaded, error, block])
}
