//! React hooks for projection state management.
//!
//! Provides `useProjection` and `usePresets` hooks with:
//! - AbortController support for cancellation
//! - Short TTL caching via the api client
//! - Proper error handling

import { useState, useEffect, useCallback, useRef } from 'react';
import { api } from '@core/api-client';
import type { ProjectionView, Preset, PresetListResponse } from './types';

// ─── useProjection ─────────────────────────────────────────────────────────

export interface UseProjectionOptions {
  /** Block ID to fetch projection for. */
  blockId: string;
  /** AbortSignal for cancellation. */
  signal?: AbortSignal;
  /** Whether to skip fetching (e.g., while editing). */
  skip?: boolean;
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
 * Uses the api client's TTL cache (30s) so concurrent reads are
 * deduplicated. The AbortSignal can be used to cancel stale requests.
 */
export function useProjection({
  blockId,
  signal,
  skip = false,
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

    async function fetchProjection() {
      setLoading(true);
      setError(null);
      try {
        const result = await api.getBlockProjection(blockId);
        // Check if the request was cancelled before updating state
        if (!cancelled && !abortRef.current?.aborted) {
          setProjection(result);
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

    fetchProjection();

    return () => {
      cancelled = true;
    };
  }, [blockId, skip]);

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
