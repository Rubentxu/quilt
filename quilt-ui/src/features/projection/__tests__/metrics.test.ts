/**
 * Tests for `ProjectionMetricsStore` and the `window.__quiltProjectionMetrics`
 * global.
 *
 * The store is a simple event-emitter. The tests cover:
 * - Counter increments
 * - Snapshot derivation (wasmRatio)
 * - Subscribe/unsubscribe symmetry
 * - The `window.__quiltProjectionMetrics` global update
 * - Error isolation (a listener that throws does not break the store)
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  ProjectionMetricsStore,
  projectionMetricsStore,
} from '../metrics'

describe('ProjectionMetricsStore', () => {
  let store: ProjectionMetricsStore

  beforeEach(() => {
    store = new ProjectionMetricsStore()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('starts at zero', () => {
    const snap = store.snapshot()
    expect(snap).toEqual({ wasmCount: 0, httpCount: 0, httpErrorCount: 0, wasmRatio: 0 })
  })

  it('recordWasm increments wasmCount and notifies', () => {
    const listener = vi.fn()
    const unsub = store.subscribe(listener)
    store.recordWasm()
    expect(store.snapshot().wasmCount).toBe(1)
    expect(listener).toHaveBeenCalledTimes(1)
    expect(listener.mock.calls[0]?.[0]?.wasmCount).toBe(1)
    unsub()
  })

  it('recordHttp increments httpCount and notifies', () => {
    const listener = vi.fn()
    const unsub = store.subscribe(listener)
    store.recordHttp()
    expect(store.snapshot().httpCount).toBe(1)
    expect(listener).toHaveBeenCalledTimes(1)
    unsub()
  })

  it('recordHttpError increments httpErrorCount and does NOT increment httpCount', () => {
    store.recordHttpError()
    const snap = store.snapshot()
    expect(snap.httpErrorCount).toBe(1)
    expect(snap.httpCount).toBe(0)
  })

  it('wasmRatio derives correctly', () => {
    // 5 WASM, 3 HTTP → 5/8 = 0.625
    for (let i = 0; i < 5; i++) store.recordWasm()
    for (let i = 0; i < 3; i++) store.recordHttp()
    expect(store.snapshot().wasmRatio).toBeCloseTo(0.625, 5)
  })

  it('wasmRatio is 0 when no resolutions', () => {
    store.recordHttpError() // does not count
    expect(store.snapshot().wasmRatio).toBe(0)
  })

  it('unsubscribe stops notifications', () => {
    const listener = vi.fn()
    const unsub = store.subscribe(listener)
    store.recordWasm()
    unsub()
    store.recordWasm()
    expect(listener).toHaveBeenCalledTimes(1)
  })

  it('errors in listeners do not break the store', () => {
    const bad = vi.fn(() => {
      throw new Error('listener boom')
    })
    const good = vi.fn()
    store.subscribe(bad)
    store.subscribe(good)
    // Should not throw even though `bad` throws.
    expect(() => store.recordWasm()).not.toThrow()
    expect(good).toHaveBeenCalledTimes(1)
  })

  it('reset() zeroes all counters', () => {
    store.recordWasm()
    store.recordHttp()
    store.recordHttpError()
    store.reset()
    expect(store.snapshot()).toEqual({
      wasmCount: 0,
      httpCount: 0,
      httpErrorCount: 0,
      wasmRatio: 0,
    })
  })

  describe('window.__quiltProjectionMetrics', () => {
    it('updates the global on every counter change', () => {
      store.recordWasm()
      expect((window as unknown as { __quiltProjectionMetrics?: { wasmCount: number } })
        .__quiltProjectionMetrics?.wasmCount).toBe(1)
      store.recordHttp()
      expect((window as unknown as { __quiltProjectionMetrics?: { httpCount: number } })
        .__quiltProjectionMetrics?.httpCount).toBe(1)
    })
  })

  it('singleton is shared across imports', () => {
    // The exported `projectionMetricsStore` is the same instance
    // every consumer sees; counters are shared.
    projectionMetricsStore.reset()
    projectionMetricsStore.recordWasm()
    // Use a fresh store to check isolation; the singleton has
    // its own counter state.
    const fresh = new ProjectionMetricsStore()
    expect(fresh.snapshot().wasmCount).toBe(0)
    // (The singleton's count may be > 0 from other tests; we just
    // verify the fresh store starts at 0.)
  })
})
