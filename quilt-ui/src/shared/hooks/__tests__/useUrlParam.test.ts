/**
 * Tests for useUrlParam — a small hook that reads a single URL
 * query param and re-renders when it changes (via popstate or a
 * manual setter).
 *
 * The hook is used by PageViewPage to read `?zoom=$blockId` so
 * the zoom feature can be driven by URL state.
 *
 * Returns a [value, setter] tuple (similar to useState) — value
 * is the current string (or null when the param is absent), and
 * setter accepts a string or null to update or remove the param.
 */
import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useUrlParam } from '@shared/hooks/useUrlParam'

// ── Test lifecycle ────────────────────────────────────────────

beforeEach(() => {
  // Each test gets a fresh URL — jsdom persists across tests otherwise.
  window.history.replaceState({}, '', '/')
})

afterEach(() => {
  vi.restoreAllMocks()
})

// ── Tests ─────────────────────────────────────────────────────

describe('useUrlParam', () => {
  it('returns the param value from the current URL on mount', () => {
    window.history.replaceState({}, '', '/page/foo?zoom=abc123')
    const { result } = renderHook(() => useUrlParam('zoom'))
    expect(result.current[0]).toBe('abc123')
  })

  it('returns null when the param is not present', () => {
    window.history.replaceState({}, '', '/page/foo')
    const { result } = renderHook(() => useUrlParam('zoom'))
    expect(result.current[0]).toBeNull()
  })

  it('returns null for a different param', () => {
    window.history.replaceState({}, '', '/page/foo?view=kanban')
    const { result } = renderHook(() => useUrlParam('zoom'))
    expect(result.current[0]).toBeNull()
  })

  it('returns the URL-decoded value (URLSearchParams.get semantics)', () => {
    // URLSearchParams.get() decodes percent-encoded values. The
    // hook mirrors that standard behavior so callers can read
    // values like "hello world" from ?zoom=hello%20world without
    // double-decoding.
    window.history.replaceState({}, '', '/page/foo?zoom=hello%20world')
    const { result } = renderHook(() => useUrlParam('zoom'))
    expect(result.current[0]).toBe('hello world')
  })

  it('updates the value when setter is called with a string', () => {
    window.history.replaceState({}, '', '/page/foo')
    const { result } = renderHook(() => useUrlParam('zoom'))

    expect(result.current[0]).toBeNull()

    act(() => {
      result.current[1]('block-xyz')
    })

    expect(result.current[0]).toBe('block-xyz')
    // The URL should also reflect the change.
    expect(window.location.search).toContain('zoom=block-xyz')
  })

  it('setter with null removes the param from the URL', () => {
    window.history.replaceState({}, '', '/page/foo?zoom=abc')
    const { result } = renderHook(() => useUrlParam('zoom'))

    expect(result.current[0]).toBe('abc')

    act(() => {
      result.current[1](null)
    })

    expect(result.current[0]).toBeNull()
    expect(window.location.search).not.toContain('zoom=')
  })

  it('setter preserves other existing query params', () => {
    window.history.replaceState({}, '', '/page/foo?view=kanban')
    const { result } = renderHook(() => useUrlParam('zoom'))

    act(() => {
      result.current[1]('block-1')
    })

    const search = window.location.search
    expect(search).toContain('zoom=block-1')
    expect(search).toContain('view=kanban')
  })

  it('setter removes the param but keeps others', () => {
    window.history.replaceState({}, '', '/page/foo?zoom=abc&view=kanban')
    const { result } = renderHook(() => useUrlParam('zoom'))

    act(() => {
      result.current[1](null)
    })

    const search = window.location.search
    expect(search).not.toContain('zoom=')
    expect(search).toContain('view=kanban')
  })

  it('responds to popstate events (browser back/forward)', () => {
    window.history.replaceState({}, '', '/page/foo')
    const { result } = renderHook(() => useUrlParam('zoom'))

    expect(result.current[0]).toBeNull()

    // Simulate the user pressing back, which restores a URL with
    // a zoom param.
    act(() => {
      window.history.replaceState({}, '', '/page/foo?zoom=back-button')
      window.dispatchEvent(new PopStateEvent('popstate'))
    })

    expect(result.current[0]).toBe('back-button')
  })

  it('handles multiple separate keys independently', () => {
    window.history.replaceState({}, '', '/page/foo?view=kanban&zoom=block-1')
    const { result: zoomHook } = renderHook(() => useUrlParam('zoom'))
    const { result: viewHook } = renderHook(() => useUrlParam('view'))

    expect(zoomHook.current[0]).toBe('block-1')
    expect(viewHook.current[0]).toBe('kanban')
  })

  it('treats an empty-string param value as null', () => {
    window.history.replaceState({}, '', '/page/foo?zoom=')
    const { result } = renderHook(() => useUrlParam('zoom'))
    expect(result.current[0]).toBeNull()
  })

  it('setter with empty string removes the param (consistency with null)', () => {
    window.history.replaceState({}, '', '/page/foo?zoom=abc')
    const { result } = renderHook(() => useUrlParam('zoom'))

    act(() => {
      result.current[1]('')
    })

    expect(result.current[0]).toBeNull()
    expect(window.location.search).not.toContain('zoom=')
  })
})
