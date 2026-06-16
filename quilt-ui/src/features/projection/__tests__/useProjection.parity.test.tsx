/**
 * Parity test for the WASM-first + HTTP-fallback `useProjection` hook.
 *
 * Verifies that:
 * - When the WASM bridge returns a result, the hook uses it and
 *   increments `wasmCount`.
 * - When the WASM bridge returns `null` (or no block is provided),
 *   the hook falls back to HTTP and increments `httpCount`.
 * - When HTTP throws, the hook captures the error and increments
 *   `httpErrorCount`.
 * - The `skip` and empty-`blockId` short-circuits still work.
 *
 * The test mocks both `wasmProjectionResolve` and `api.getBlockProjection`
 * so we can deterministically control which path serves the request.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useProjection, projectionMetricsStore } from '../hooks'
import type { ProjectionView, ProjectionConflict, Decoration, LinkView } from '../types'

// ── Mocks ─────────────────────────────────────────────────────────────

const { mockWasmProjectionResolve, mockGetBlockProjection } = vi.hoisted(() => {
  return {
    mockWasmProjectionResolve: vi.fn(),
    mockGetBlockProjection: vi.fn(),
  }
})

vi.mock('@core/wasm-bridge/wasm-loader', () => ({
  wasmProjectionResolve: mockWasmProjectionResolve,
  ping: () => true,
  get_version: () => 'test',
}))

vi.mock('@core/api-client', () => ({
  api: {
    getBlockProjection: mockGetBlockProjection,
    invalidateAll: vi.fn(),
  },
}))

// ── Test fixtures ────────────────────────────────────────────────────

const SAMPLE_VIEW: ProjectionView = {
  text: 'Buy milk',
  links: [] as LinkView[],
  children: [],
  decorations: [
    {
      kind: 'task-checkbox',
      target: 'status',
      value: 'done',
      weight: 100,
    },
  ] as Decoration[],
  conflicts: [] as ProjectionConflict[],
  properties: {
    type: 'task',
    status: 'done',
    projection: 'task',
  },
}

const SAMPLE_BLOCK = {
  id: 'b1',
  pageId: 'p1',
  content: 'Buy milk',
  properties: [
    { key: 'type', value: 'task', type: 'string' as const },
    { key: 'status', value: 'done', type: 'string' as const },
  ],
}

beforeEach(() => {
  mockWasmProjectionResolve.mockReset()
  mockGetBlockProjection.mockReset()
  projectionMetricsStore.reset()
})

afterEach(() => {
  vi.clearAllMocks()
})

describe('useProjection — WASM-first + HTTP-fallback (ADR-0028)', () => {
  it('uses WASM when block is provided and bridge returns a result', async () => {
    mockWasmProjectionResolve.mockReturnValue({
      view: SAMPLE_VIEW,
      contractId: 'task',
      hadConflict: false,
    })

    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK }),
    )

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.projection).toEqual(SAMPLE_VIEW)
    expect(result.current.error).toBeNull()
    expect(mockWasmProjectionResolve).toHaveBeenCalledTimes(1)
    // The HTTP call should NOT have fired (WASM served the request).
    expect(mockGetBlockProjection).not.toHaveBeenCalled()
    // The metrics should reflect the WASM path.
    expect(projectionMetricsStore.snapshot().wasmCount).toBe(1)
    expect(projectionMetricsStore.snapshot().httpCount).toBe(0)
  })

  it('falls back to HTTP when WASM returns null', async () => {
    mockWasmProjectionResolve.mockReturnValue(null)
    mockGetBlockProjection.mockResolvedValue(SAMPLE_VIEW)

    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK }),
    )

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.projection).toEqual(SAMPLE_VIEW)
    expect(mockGetBlockProjection).toHaveBeenCalledWith('b1')
    expect(projectionMetricsStore.snapshot().httpCount).toBe(1)
    expect(projectionMetricsStore.snapshot().wasmCount).toBe(0)
  })

  it('goes HTTP-only when no block is provided', async () => {
    mockGetBlockProjection.mockResolvedValue(SAMPLE_VIEW)

    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1' }),
    )

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.projection).toEqual(SAMPLE_VIEW)
    // WASM should NOT have been called.
    expect(mockWasmProjectionResolve).not.toHaveBeenCalled()
    expect(mockGetBlockProjection).toHaveBeenCalledTimes(1)
    expect(projectionMetricsStore.snapshot().httpCount).toBe(1)
  })

  it('captures the error and increments httpErrorCount on HTTP failure', async () => {
    mockWasmProjectionResolve.mockReturnValue(null)
    const apiError = new Error('boom')
    mockGetBlockProjection.mockRejectedValue(apiError)

    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK }),
    )

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.projection).toBeNull()
    expect(result.current.error).toBe(apiError)
    expect(projectionMetricsStore.snapshot().httpErrorCount).toBe(1)
    expect(projectionMetricsStore.snapshot().httpCount).toBe(0)
  })

  it('captures a non-Error HTTP rejection as a new Error', async () => {
    mockWasmProjectionResolve.mockReturnValue(null)
    mockGetBlockProjection.mockRejectedValue('string error')

    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK }),
    )

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBeInstanceOf(Error)
    expect(result.current.error?.message).toBe('string error')
  })

  it('short-circuits when skip is true', async () => {
    const { result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK, skip: true }),
    )

    expect(result.current.projection).toBeNull()
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBeNull()
    expect(mockWasmProjectionResolve).not.toHaveBeenCalled()
    expect(mockGetBlockProjection).not.toHaveBeenCalled()
  })

  it('short-circuits when blockId is empty', async () => {
    const { result } = renderHook(() =>
      useProjection({ blockId: '', block: SAMPLE_BLOCK }),
    )

    expect(result.current.projection).toBeNull()
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBeNull()
    expect(mockWasmProjectionResolve).not.toHaveBeenCalled()
  })

  it('does not update state after unmount (no late setState warnings)', async () => {
    // A slow HTTP call that resolves after the hook unmounts.
    let resolveHttp!: (v: ProjectionView) => void
    mockGetBlockProjection.mockImplementation(
      () => new Promise<ProjectionView>((res) => { resolveHttp = res }),
    )
    mockWasmProjectionResolve.mockReturnValue(null)

    const { unmount, result } = renderHook(() =>
      useProjection({ blockId: 'b1', block: SAMPLE_BLOCK }),
    )

    // The hook has started the HTTP call but it hasn't resolved yet.
    expect(result.current.loading).toBe(true)

    unmount()

    // Resolve the HTTP call after unmount. The hook's `cancelled` flag
    // is set, so the `recordHttp` call is gated (it would otherwise
    // double-count a "successful" resolution that no consumer saw).
    // The important assertion is that no React warning was logged —
    // the test framework would surface it as a failure otherwise.
    await act(async () => {
      resolveHttp(SAMPLE_VIEW)
    })

    // After unmount, the httpCount was NOT recorded (the hook's
    // `cancelled` guard short-circuited before `recordHttp`).
    expect(projectionMetricsStore.snapshot().httpCount).toBe(0)
  })

  it('records multiple resolutions across re-renders', async () => {
    mockWasmProjectionResolve.mockReturnValue({
      view: SAMPLE_VIEW,
      contractId: 'task',
      hadConflict: false,
    })

    const { rerender } = renderHook(
      ({ blockId }) => useProjection({ blockId, block: SAMPLE_BLOCK }),
      { initialProps: { blockId: 'b1' } },
    )

    await waitFor(() => {
      expect(projectionMetricsStore.snapshot().wasmCount).toBe(1)
    })

    rerender({ blockId: 'b2' })

    await waitFor(() => {
      expect(projectionMetricsStore.snapshot().wasmCount).toBe(2)
    })
  })
})
