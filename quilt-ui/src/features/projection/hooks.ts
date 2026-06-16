//! React hooks for projection state management.
//!
//! Provides `useProjection` and `usePresets` hooks with:
//! - AbortController support for cancellation
//! - Short TTL caching via the api client
//! - Proper error handling
//!
//! Also exports `useProjectionMetrics` (ADR-0028) for the debug panel.

import { useState, useEffect, useCallback, useRef, useSyncExternalStore } from 'react';
import { api } from '@core/api-client';
import { wasmProjectionResolve } from '@core/wasm-bridge/wasm-loader';
import type { Block, ProjectionView, Preset, PresetListResponse } from './types';
import { projectionMetricsStore, type ProjectionMetrics } from './metrics';

// ─── useProjectionMetrics (ADR-0028) ───────────────────────────────────────

/**
 * React hook that subscribes to `ProjectionMetricsStore` and returns
 * the current snapshot. Re-renders on every counter change.
 *
 * Used by the debug panel (gated by `VITE_DEBUG_PANEL=true`) and any
 * other surface that wants to display the WASM vs HTTP ratio.
 */
export function useProjectionMetrics(): ProjectionMetrics {
  return useSyncExternalStore(
    (listener) => projectionMetricsStore.subscribe(listener),
    () => projectionMetricsStore.snapshot(),
    () => projectionMetricsStore.snapshot(),
  )
}

// ─── useProjection ─────────────────────────────────────────────────────────

export interface UseProjectionOptions {
  /** Block ID to fetch projection for. */
  blockId: string;
  /** AbortSignal for cancellation. */
  signal?: AbortSignal;
  /** Whether to skip fetching (e.g., while editing). */
  skip?: boolean;
  /**
   * Optional React-side `Block` object. When provided, the hook
   * tries the WASM path first (no network round-trip). When the
   * WASM path returns `null` (WASM not loaded, returned null, or
   * threw), the hook falls back to HTTP via `api.getBlockProjection`.
   * When `block` is undefined, the hook goes HTTP-only.
   *
   * This parameter is optional; consumers that don't have the
   * block locally (e.g., test mocks) can omit it and the HTTP
   * path runs as before.
   */
  block?: Block;
}

export interface UseProjectionResult {
  /** The resolved projection view, or null while loading. */
  projection: ProjectionView | null;
  /** Whether a fetch is in progress. */
  loading: boolean;
  /** Error if the fetch failed. */
  error: Error | null;
}

/**
 * Fetch and cache a block's projection view.
 *
 * Strategy (ADR-0028): WASM-first, HTTP-fallback.
 * 1. If a `block` is provided AND the WASM module has the
 *    `projection_resolve` export, try WASM first (no network).
 * 2. If WASM returns a result, use it (no HTTP call).
 * 3. If WASM returns null (not loaded, error, or invalid block),
 *    fall back to HTTP via `api.getBlockProjection`.
 * 4. If `block` is undefined, go HTTP-only.
 *
 * Metrics: every successful resolution records into
 * `ProjectionMetricsStore` (`recordWasm` or `recordHttp`).
 * HTTP errors record into `recordHttpError`. The metrics are
 * exposed via `useProjectionMetrics` (for the debug panel) and
 * `window.__quiltProjectionMetrics` (for E2E tests).
 *
 * The existing 3 test mocks for `BlockRow` and `PageView.zoom`
 * continue to work without modification (they replace
 * `useProjection` entirely via `vi.mock`).
 */
export function useProjection({
  blockId,
  signal,
  skip = false,
  block,
}: UseProjectionOptions): UseProjectionResult {
  const [projection, setProjection] = useState<ProjectionView | null>(null);
  const [loading, setLoading] = useState(!skip && !!blockId);
  const [error, setError] = useState<Error | null>(null);
  const abortRef = useRef(signal);

  useEffect(() => {
    // Sync signal ref for cleanup
    abortRef.current = signal;
  }, [signal]);

  useEffect(() => {
    if (skip || !blockId) {
      setProjection(null);
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;

    async function resolveProjection() {
      setLoading(true);
      setError(null);

      // 1. WASM-first path (only when the React-side block is available)
      if (block) {
        try {
          const wasmResult = wasmProjectionResolve(block)
          if (wasmResult && !cancelled && !abortRef.current?.aborted) {
            setProjection(wasmResult.view)
            projectionMetricsStore.recordWasm()
            setLoading(false)
            return // HTTP call is NOT fired
          }
          // WASM returned null → fall through to HTTP
        } catch {
          // Defensive: the bridge already swallows errors, but if
          // something exotic happens, fall through to HTTP.
        }
      }

      // 2. HTTP-fallback path
      try {
        const result = await api.getBlockProjection(blockId)
        if (!cancelled && !abortRef.current?.aborted) {
          setProjection(result)
          projectionMetricsStore.recordHttp()
        }
      } catch (err) {
        if (!cancelled && !abortRef.current?.aborted) {
          setError(err instanceof Error ? err : new Error(String(err)))
          projectionMetricsStore.recordHttpError()
        }
      } finally {
        if (!cancelled && !abortRef.current?.aborted) {
          setLoading(false)
        }
      }
    }

    resolveProjection();

    return () => {
      cancelled = true;
    };
  }, [blockId, skip, block]);

  return { projection, loading, error };
}

// ─── usePresets ────────────────────────────────────────────────────────────

export interface UsePresetsOptions {
  /** AbortSignal for cancellation. */
  signal?: AbortSignal;
  /** Whether to skip fetching. */
  skip?: boolean;
}

export interface UsePresetsResult {
  /** All available presets. */
  presets: Preset[];
  /** Whether a fetch is in progress. */
  loading: boolean;
  /** Error if the fetch failed. */
  error: Error | null;
}

/**
 * Fetch and cache the list of available property presets.
 *
 * The preset list changes rarely (only when new presets are added to the
 * server), so a longer cache TTL is appropriate here.
 */
export function usePresets({
  signal,
  skip = false,
}: UsePresetsOptions = {}): UsePresetsResult {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loading, setLoading] = useState(!skip);
  const [error, setError] = useState<Error | null>(null);
  const abortRef = useRef(signal);

  useEffect(() => {
    abortRef.current = signal;
  }, [signal]);

  useEffect(() => {
    if (skip) {
      setPresets([]);
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;

    async function fetchPresets() {
      setLoading(true);
      setError(null);
      try {
        const result: PresetListResponse = await api.listPresets();
        if (!cancelled && !abortRef.current?.aborted) {
          setPresets(result.presets);
        }
      } catch (err) {
        if (!cancelled && !abortRef.current?.aborted) {
          setError(err instanceof Error ? err : new Error(String(err)));
        }
      } finally {
        if (!cancelled && !abortRef.current?.aborted) {
          setLoading(false);
        }
      }
    }

    fetchPresets();

    return () => {
      cancelled = true;
    };
  }, [skip]);

  return { presets, loading, error };
}

// ─── usePresetApplication ───────────────────────────────────────────────────

export interface ApplyPresetParams {
  /** Block ID to apply the preset to. */
  blockId: string;
  /** Preset identifier (e.g., "/TODO"). */
  presetId: string;
  /** Arguments required by the preset (e.g., { date: "2024-01-15" }). */
  args?: Record<string, string>;
}

/**
 * Hook that provides a function to apply a preset to a block.
 *
 * Note: The actual `applyPreset` API endpoint (POST /api/v1/blocks/:id/presets)
 * may not exist yet on the server. This hook returns a function that
 * invalidates the projection cache for the block, assuming the preset
 * was applied via a slash command that updated properties directly.
 */
export function usePresetApplication() {
  const applyPreset = useCallback(
    async ({ blockId }: ApplyPresetParams): Promise<void> => {
      // TODO: When the POST /api/v1/blocks/:id/presets endpoint is added,
      // implement the actual API call here. For now, we invalidate the
      // projection cache so the next read will reflect any property changes
      // that may have been applied via the slash command handler.
      //
      // The slashRegistry's applyPreset handler calls this and then
      // updates block properties via api.setBlockProperty for each
      // property in the preset patch.
      api.invalidateAll();
    },
    [],
  );

  return { applyPreset };
}

// Re-export types
export type { ProjectionView, Preset };

// Re-export the metrics store for callers that want to record
// metrics without going through the hook (e.g., E2E tests).
export { projectionMetricsStore, type ProjectionMetrics } from './metrics';
