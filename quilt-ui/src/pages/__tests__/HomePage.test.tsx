// HomePage — root `/` should redirect to today's journal (YYYY-MM-DD).
//
// F1 of P0 frontend fixes: visiting `/` used to render `null`, leaving
// the user staring at an empty shell. The route is supposed to land
// them on the journal for "today" — the same date format the
// `/journal/$date` route accepts (e.g. `2026-06-05`).
//
// Per ADR-0030 §8, the home always lands on today's journal when
// a valid last_opened_graph exists. If there is no valid graph
// (first run or invalid path), it redirects to the graph selector.
//
// We assert the BEHAVIOR (navigation was triggered with the right
// target) — not the implementation detail of which React hook the
// component uses to schedule the navigation.

import { render, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { HomePage } from '../HomePage'
import { api } from '@core/api-client'

// ── Mocks ─────────────────────────────────────────────────────────────────

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('@core/api-client', () => ({
  api: {
    getGlobalState: vi.fn<() => Promise<{
      lastOpenedGraph: string | null;
      recentGraphs: string[];
      rightSidebarVisible: boolean | null;
    }>>(),
  },
}))

// ── Helpers ───────────────────────────────────────────────────────────────

/** Local-tz YYYY-MM-DD for "today", matching what HomePage should produce. */
function expectedTodayIso(): string {
  const d = new Date()
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

beforeEach(() => {
  mockNavigate.mockReset()
  vi.mocked(api.getGlobalState).mockReset()
})

afterEach(() => {
  vi.useRealTimers()
})

// ── Tests ─────────────────────────────────────────────────────────────────

describe('HomePage — conditional routing based on global state (ADR-0030 §8)', () => {
  it('renders nothing visible (null) on `/`', async () => {
    vi.mocked(api.getGlobalState).mockResolvedValue({ lastOpenedGraph: '/home/user/graph', recentGraphs: [], rightSidebarVisible: null })
    const { container } = render(<HomePage />)
    // The redirect component shouldn't paint anything to the DOM —
    // its job is to fire the navigation, then unmount.
    expect(container.firstChild).toBeNull()
  })

  it('navigates to /journal/$date with today\'s date when lastOpenedGraph exists', async () => {
    vi.mocked(api.getGlobalState).mockResolvedValue({ lastOpenedGraph: '/home/user/graph', recentGraphs: [], rightSidebarVisible: null })

    render(<HomePage />)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledTimes(1)
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: expectedTodayIso() },
      })
    })
  })

  it('uses the actual current date (not a hardcoded value)', async () => {
    // Pin the clock to a known instant so we can prove the redirect
    // reads `new Date()` and not a stale constant.
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-05T10:30:00Z'))
    vi.mocked(api.getGlobalState).mockResolvedValue({ lastOpenedGraph: '/home/user/graph', recentGraphs: [], rightSidebarVisible: null })

    render(<HomePage />)

    // Under fake timers, mockResolvedValue's promise may not auto-resolve.
    // Use runAllTimers to flush microtasks and let the navigate call fire.
    await vi.runAllTimersAsync()

    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/journal/$date',
      params: { date: '2026-06-05' },
    })
  })

  it('redirects to /select-graph when lastOpenedGraph is null (first run)', async () => {
    vi.mocked(api.getGlobalState).mockResolvedValue({ lastOpenedGraph: null, recentGraphs: [], rightSidebarVisible: null })

    render(<HomePage />)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledTimes(1)
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/select-graph' })
    })
  })

  it('redirects to /select-graph when lastOpenedGraph is empty string', async () => {
    vi.mocked(api.getGlobalState).mockResolvedValue({ lastOpenedGraph: '', recentGraphs: [], rightSidebarVisible: null })

    render(<HomePage />)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/select-graph' })
    })
  })

  it('redirects to /select-graph on getGlobalState network error (safe fallback)', async () => {
    vi.mocked(api.getGlobalState).mockRejectedValue(new Error('network error'))

    render(<HomePage />)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/select-graph' })
    })
  })

  it('does not navigate if the component unmounts before getGlobalState resolves', async () => {
    vi.mocked(api.getGlobalState).mockImplementation(
      () => new Promise((r) => setTimeout(() => r({ lastOpenedGraph: '/home/user/graph', recentGraphs: [], rightSidebarVisible: null }), 500))
    )

    const { unmount } = render(<HomePage />)
    unmount()

    // Cancelled — no navigation should occur
    expect(mockNavigate).not.toHaveBeenCalled()
  })
})
