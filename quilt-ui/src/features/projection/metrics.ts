//! Projection metrics store — WASM vs HTTP usage counters.
//!
//! A simple event-emitter singleton that holds three counters and
//! notifies subscribers on change. Used by:
//!
//! - The `useProjection` hook (slice #5) to record which path served
//!   each resolution.
//! - The `useProjectionMetrics` hook for the debug panel.
//! - The `window.__quiltProjectionMetrics` global for E2E tests.
//!
//! The store is intentionally simple — in-memory only, no
//! `localStorage`, no `IndexedDB`, no network. A future slice may add
//! persistence (the slice notes this in
//! `openspec/.../specs/projection-metrics/spec.md`).

/** Snapshot of the projection metrics. */
export interface ProjectionMetrics {
  /** Successful WASM resolutions. */
  wasmCount: number
  /** Successful HTTP resolutions. */
  httpCount: number
  /** HTTP errors (network failure, 5xx, 4xx). */
  httpErrorCount: number
  /** wasm / (wasm + http) ratio in [0, 1]; 0 if no resolutions. */
  wasmRatio: number
}

const ZERO_METRICS: ProjectionMetrics = Object.freeze({
  wasmCount: 0,
  httpCount: 0,
  httpErrorCount: 0,
  wasmRatio: 0,
})

export type MetricsListener = (snapshot: ProjectionMetrics) => void

/**
 * In-process projection metrics store.
 *
 * Singleton (one per browser tab). Subscribers are notified
 * synchronously after every counter change. Listeners MUST be
 * idempotent (they may be called multiple times for the same
 * snapshot if they record multiple times in a row).
 */
export class ProjectionMetricsStore {
  private wasmCount = 0
  private httpCount = 0
  private httpErrorCount = 0
  private listeners = new Set<MetricsListener>()

  /** Record a successful WASM resolution. */
  recordWasm(): void {
    this.wasmCount += 1
    this.notify()
  }

  /** Record a successful HTTP resolution. */
  recordHttp(): void {
    this.httpCount += 1
    this.notify()
  }

  /** Record an HTTP error (network failure, 5xx, 4xx). */
  recordHttpError(): void {
    this.httpErrorCount += 1
    this.notify()
  }

  /** Read a snapshot of the current counters. */
  snapshot(): ProjectionMetrics {
    const total = this.wasmCount + this.httpCount
    return {
      wasmCount: this.wasmCount,
      httpCount: this.httpCount,
      httpErrorCount: this.httpErrorCount,
      wasmRatio: total > 0 ? this.wasmCount / total : 0,
    }
  }

  /** Subscribe to changes. Returns an unsubscribe function. */
  subscribe(listener: MetricsListener): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  /** Test helper: reset all counters to zero. */
  reset(): void {
    this.wasmCount = 0
    this.httpCount = 0
    this.httpErrorCount = 0
    this.notify()
  }

  private notify(): void {
    const snapshot = this.snapshot()
    // Update the global for E2E tests (the global is set once at
    // module load and updated here on every counter change).
    if (typeof window !== 'undefined') {
      ;(window as unknown as { __quiltProjectionMetrics?: ProjectionMetrics }).__quiltProjectionMetrics =
        snapshot
    }
    for (const listener of this.listeners) {
      try {
        listener(snapshot)
      } catch {
        // Listener errors must not break the store; ignore.
      }
    }
  }
}

/** The single shared metrics store instance. */
export const projectionMetricsStore = new ProjectionMetricsStore()

// Set the initial value of the E2E-test global on module load. The
// store updates it on every counter change.
if (typeof window !== 'undefined') {
  ;(window as unknown as { __quiltProjectionMetrics?: ProjectionMetrics }).__quiltProjectionMetrics =
    ZERO_METRICS
}

// Type-augment window for consumers.
declare global {
  interface Window {
    __quiltProjectionMetrics?: ProjectionMetrics
  }
}
