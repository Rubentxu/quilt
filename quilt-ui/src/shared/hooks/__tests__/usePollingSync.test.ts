/**
 * Tests for usePollingSync — polls the API for block updates on an interval.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { usePollingSync } from '@shared/hooks/usePollingSync'
import type { Block } from '@shared/types/api'

// Mock the api module
vi.mock('@core/api-client', () => ({
  api: {
    getPageBlocks: vi.fn(),
  },
}))

import { api } from '@core/api-client'

const mockBlock = (id: string): Block => ({
  id,
  pageId: 'p1',
  pageName: 'test-page',
  content: `block ${id}`,
  blockType: 'paragraph',
  marker: null,
  priority: null,
  parentId: null,
  order: 0,
  level: 1,
  collapsed: false,
  properties: [],
  createdAt: '2026-01-01',
  updatedAt: '2026-01-01',
})

describe('usePollingSync', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('does not start polling when enabled is false', () => {
    const onBlocksChanged = vi.fn()
    vi.mocked(api.getPageBlocks).mockResolvedValue([])

    renderHook(() =>
      usePollingSync({
        pageName: 'test',
        interval: 1000,
        onBlocksChanged,
        enabled: false,
      }),
    )

    act(() => {
      vi.advanceTimersByTime(2000)
    })

    expect(api.getPageBlocks).not.toHaveBeenCalled()
  })

  it('does not start polling when pageName is null', () => {
    const onBlocksChanged = vi.fn()

    renderHook(() =>
      usePollingSync({
        pageName: null,
        interval: 1000,
        onBlocksChanged,
        enabled: true,
      }),
    )

    act(() => {
      vi.advanceTimersByTime(2000)
    })

    expect(api.getPageBlocks).not.toHaveBeenCalled()
  })

  it('calls api.getPageBlocks on the interval', async () => {
    const onBlocksChanged = vi.fn()
    const blocks: Block[] = [mockBlock('b1')]
    vi.mocked(api.getPageBlocks).mockResolvedValue(blocks)

    renderHook(() =>
      usePollingSync({
        pageName: 'test-page',
        interval: 5000,
        onBlocksChanged,
        enabled: true,
      }),
    )

    // First call on mount is immediate (setInterval fires after interval)
    await act(async () => {
      vi.advanceTimersByTimeAsync(5000)
    })

    expect(api.getPageBlocks).toHaveBeenCalledWith('test-page')
    expect(api.getPageBlocks).toHaveBeenCalledTimes(1)
  })

  it('calls onBlocksChanged with the fetched blocks', async () => {
    const onBlocksChanged = vi.fn()
    const blocks: Block[] = [mockBlock('b1'), mockBlock('b2')]
    vi.mocked(api.getPageBlocks).mockResolvedValue(blocks)

    renderHook(() =>
      usePollingSync({
        pageName: 'test-page',
        interval: 3000,
        onBlocksChanged,
        enabled: true,
      }),
    )

    await act(async () => {
      vi.advanceTimersByTimeAsync(3000)
    })

    expect(onBlocksChanged).toHaveBeenCalledTimes(1)
    expect(onBlocksChanged).toHaveBeenCalledWith(blocks)
  })

  it('polls repeatedly on each interval tick', async () => {
    const onBlocksChanged = vi.fn()
    vi.mocked(api.getPageBlocks).mockResolvedValue([])

    renderHook(() =>
      usePollingSync({
        pageName: 'test-page',
        interval: 1000,
        onBlocksChanged,
        enabled: true,
      }),
    )

    // Advance timer by one tick at a time to let async callbacks resolve
    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })
    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })
    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })

    expect(api.getPageBlocks).toHaveBeenCalledTimes(3)
  })

  it('clears interval on unmount', () => {
    const onBlocksChanged = vi.fn()
    vi.mocked(api.getPageBlocks).mockResolvedValue([])

    const { unmount } = renderHook(() =>
      usePollingSync({
        pageName: 'test-page',
        interval: 1000,
        onBlocksChanged,
        enabled: true,
      }),
    )

    unmount()

    act(() => {
      vi.advanceTimersByTime(5000)
    })

    // Should not have been called after unmount (initial call already happened)
    // Actually the first interval call happens at 1000ms but we unmounted before
    expect(api.getPageBlocks).not.toHaveBeenCalled()
  })

  it('handles API errors silently', async () => {
    const onBlocksChanged = vi.fn()
    vi.mocked(api.getPageBlocks).mockRejectedValue(new Error('Network error'))

    // Should not throw
    expect(() => {
      renderHook(() =>
        usePollingSync({
          pageName: 'test-page',
          interval: 1000,
          onBlocksChanged,
          enabled: true,
        }),
      )
    }).not.toThrow()

    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })

    // onBlocksChanged should not be called on error
    expect(onBlocksChanged).not.toHaveBeenCalled()
  })

  it('re-starts polling when pageName changes', async () => {
    const onBlocksChanged = vi.fn()
    vi.mocked(api.getPageBlocks).mockResolvedValue([])

    const { rerender } = renderHook(
      ({ pageName }) =>
        usePollingSync({
          pageName,
          interval: 1000,
          onBlocksChanged,
          enabled: true,
        }),
      { initialProps: { pageName: 'page-1' as string | null } },
    )

    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })

    expect(api.getPageBlocks).toHaveBeenCalledWith('page-1')

    // Change the page name
    rerender({ pageName: 'page-2' })

    await act(async () => {
      vi.advanceTimersByTimeAsync(1000)
    })

    expect(api.getPageBlocks).toHaveBeenCalledWith('page-2')
  })
})
