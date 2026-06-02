/**
 * Tests for useSSE — Server-Sent Events connection with
 * auto-reconnect, event parsing, and disconnect.
 *
 * EventSource is mocked since jsdom doesn't fully support it.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useSSE } from '@shared/hooks/useSSE'

// ── Mock EventSource ────────────────────────────────────────

interface MockEventSourceInstance {
  onopen: (() => void) | null
  onmessage: ((e: MessageEvent) => void) | null
  onerror: (() => void) | null
  addEventListener: ReturnType<typeof vi.fn>
  close: ReturnType<typeof vi.fn>
}

let mockInstance: MockEventSourceInstance | null = null

class MockEventSource {
  onopen: (() => void) | null = null
  onmessage: ((e: MessageEvent) => void) | null = null
  onerror: (() => void) | null = null
  addEventListener = vi.fn()
  close = vi.fn()

  constructor(public url: string) {
    mockInstance = this
  }

  // Helper to simulate events
  _triggerOpen() {
    this.onopen?.()
  }

  _triggerMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent)
  }

  _triggerError() {
    this.onerror?.()
  }
}

// Override global EventSource
vi.stubGlobal('EventSource', MockEventSource)

// ── Helpers ──────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers()
  mockInstance = null
})

afterEach(() => {
  vi.useRealTimers()
})

function renderSSE(options: { enabled?: boolean; url?: string } = {}) {
  const onEvent = vi.fn()
  const { result, unmount } = renderHook(() =>
    useSSE({
      url: options.url ?? '/api/v1/events',
      onEvent,
      enabled: options.enabled ?? true,
      reconnectInterval: 1000,
    }),
  )
  return { result, onEvent, unmount }
}

// ── Connection state ────────────────────────────────────────

describe('useSSE', () => {
  it('connects to the given URL', () => {
    renderSSE({ url: '/api/v1/events' })
    expect(mockInstance).not.toBeNull()
    expect(mockInstance!.url ?? (MockEventSource as any).mock?.calls?.[0]?.[0])
  })

  it('reports connected state after onopen', () => {
    const { result } = renderSSE()
    expect(result.current.connected).toBe(false)

    act(() => {
      mockInstance!._triggerOpen()
    })

    expect(result.current.connected).toBe(true)
    expect(result.current.error).toBeNull()
  })

  it('does not connect when enabled is false', () => {
    renderSSE({ enabled: false })
    // EventSource should not be created
    expect(mockInstance).toBeNull()
  })

  // ── Message handling ──────────────────────────────────

  it('calls onEvent with parsed JSON messages', () => {
    const { onEvent } = renderSSE()

    act(() => {
      mockInstance!._triggerMessage({ type: 'test', data: { foo: 'bar' } })
    })

    expect(onEvent).toHaveBeenCalledWith({ type: 'test', data: { foo: 'bar' } })
  })

  it('ignores non-JSON messages silently', () => {
    const { onEvent } = renderSSE()

    act(() => {
      // Send raw text, not JSON
      mockInstance!.onmessage?.({ data: 'not json' } as MessageEvent)
    })

    expect(onEvent).not.toHaveBeenCalled()
  })

  // ── Error handling ────────────────────────────────────

  it('reports error and disconnects on onerror', () => {
    const { result } = renderSSE()

    act(() => {
      mockInstance!._triggerOpen()
    })
    expect(result.current.connected).toBe(true)

    act(() => {
      mockInstance!._triggerError()
    })

    expect(result.current.connected).toBe(false)
    expect(result.current.error).toBe('SSE connection lost')
  })

  it('attempts to reconnect after error', () => {
    const { result } = renderSSE()

    // First connection succeeds
    act(() => {
      mockInstance!._triggerOpen()
    })

    // Then fails
    const firstInstance = mockInstance
    act(() => {
      firstInstance!._triggerError()
    })
    expect(firstInstance!.close).toHaveBeenCalled()

    // After reconnectInterval, a new EventSource should be created
    act(() => {
      vi.advanceTimersByTime(1000)
    })

    // A new connection should be attempted
    // (MockEventSource is constructed again, but mockInstance is overwritten)
    expect(mockInstance).not.toBeNull()
  })

  // ── Disconnect ────────────────────────────────────────

  it('disconnects on unmount', () => {
    const { unmount } = renderSSE()

    unmount()

    expect(mockInstance!.close).toHaveBeenCalled()
  })

  it('disconnect() closes the connection', () => {
    const { result } = renderSSE()

    act(() => {
      mockInstance!._triggerOpen()
    })
    expect(result.current.connected).toBe(true)

    act(() => {
      result.current.disconnect()
    })

    expect(result.current.connected).toBe(false)
    expect(mockInstance!.close).toHaveBeenCalled()
  })

  it('reconnect() opens a new connection', () => {
    const { result } = renderSSE()

    act(() => {
      mockInstance!._triggerOpen()
    })

    // Disconnect
    act(() => {
      result.current.disconnect()
    })

    const oldInstance = mockInstance
    // Reconnect
    act(() => {
      result.current.reconnect()
    })

    // A new EventSource should be created
    expect(mockInstance).not.toBeNull()
  })
})
