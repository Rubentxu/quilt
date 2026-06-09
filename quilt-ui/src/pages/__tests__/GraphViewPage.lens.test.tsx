// GraphViewPage — Graph Lens V1 selector.
//
// The graph view now exposes a lens selector above the canvas with
// four options: "All" (default, no focus), "Page context" (focus on
// the current page), "Block subtree" (focus on a specific block —
// not used in V1 default view), and "Property filter" (focus on
// blocks with a specific property key).
//
// In V1, the only lens that needs a focus argument is "Property
// filter" (we prompt the user for a key). The other two either
// need no focus ("All") or operate on the implicit current page
// ("Page context" — which in V1 we just call the lens endpoint
// without a `focus` param, keeping it the same as "All" since we
// don't have a "current page" concept at the graph page level).
// We expose the buttons and a "data-active" attribute that the
// tests can assert on, and the API call shape is the contract
// under test.
//
// Tests assert BEHAVIOR — what the user sees and what the API
// receives — not the implementation details of which React hooks
// drive the fetch.

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { GraphViewPage } from '../GraphViewPage'

// ── Mocks ─────────────────────────────────────────────────────────

// `vi.mock` factories are hoisted to the top of the file, so any
// mock objects they reference must also be hoisted via
// `vi.hoisted` — otherwise the factory captures a `const` that
// hasn't been initialised yet.
const { mockApi, mockNavigate, mockOpenTab } = vi.hoisted(() => ({
  mockApi: {
    listPages: vi.fn(),
    getPageBacklinks: vi.fn(),
    getGraphLens: vi.fn(),
  },
  mockNavigate: vi.fn(),
  mockOpenTab: vi.fn(),
}))

vi.mock('@core/api-client', () => ({
  api: mockApi,
  QuiltApiError: class extends Error {
    constructor(public status: number, public code: string, public detail: string) {
      super(detail)
      this.name = 'QuiltApiError'
    }
  },
  getEventsUrl: () => '/api/v1/events',
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('@shared/contexts/TabsContext', () => ({
  useTabs: () => ({ openTab: mockOpenTab }),
}))

// ── Setup ─────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  // Default: empty graph so the test doesn't blow up on canvas init.
  mockApi.listPages.mockResolvedValue([])
  mockApi.getPageBacklinks.mockResolvedValue([])
  mockApi.getGraphLens.mockResolvedValue({
    focus: null,
    depth: 1,
    nodes: [],
    edges: [],
  })
})

// ── Tests ─────────────────────────────────────────────────────────

describe('GraphViewPage — lens selector', () => {
  it('renders all four lens options', () => {
    render(<GraphViewPage />)
    // The four lens options are radio buttons (mutually exclusive
    // selection is the right ARIA semantics). Query by role+name.
    expect(screen.getByRole('radio', { name: 'All' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Page context' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Block subtree' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Property filter' })).toBeInTheDocument()
  })

  it('marks "All" as the active lens on first render', () => {
    render(<GraphViewPage />)
    const allBtn = screen.getByRole('radio', { name: 'All' })
    expect(allBtn.getAttribute('data-active')).toBe('true')

    // The other lenses must not be active.
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('false')
    expect(screen.getByRole('radio', { name: 'Block subtree' }).getAttribute('data-active')).toBe('false')
    expect(screen.getByRole('radio', { name: 'Property filter' }).getAttribute('data-active')).toBe('false')
  })

  it('switches the active lens when a different option is clicked', () => {
    render(<GraphViewPage />)

    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))

    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('false')
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('true')
  })

  it('does NOT call getGraphLens on initial mount for the "All" lens', async () => {
    // "All" is the default and is implemented by the existing
    // listPages+backlinks pipeline, NOT by calling the lens
    // endpoint. We assert this so a future "always call lens" refactor
    // is a conscious decision.
    render(<GraphViewPage />)

    // Wait a tick for effects to flush.
    await waitFor(() => {
      expect(mockApi.listPages).toHaveBeenCalled()
    })
    expect(mockApi.getGraphLens).not.toHaveBeenCalled()
  })

  it('calls getGraphLens when the "Page context" lens is activated', async () => {
    render(<GraphViewPage />)
    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))

    await waitFor(() => {
      expect(mockApi.getGraphLens).toHaveBeenCalled()
    })

    // "Page context" in V1 just refreshes with no focus — the focus
    // argument is undefined. The depth defaults to 1.
    const call = mockApi.getGraphLens.mock.calls[0][0]
    expect(call.focus).toBeUndefined()
    expect(call.depth).toBe(1)
  })

  it('calls getGraphLens with depth=2 when "Block subtree" is activated', async () => {
    render(<GraphViewPage />)
    fireEvent.click(screen.getByRole('radio', { name: 'Block subtree' }))

    await waitFor(() => {
      expect(mockApi.getGraphLens).toHaveBeenCalled()
    })
    const call = mockApi.getGraphLens.mock.calls[0][0]
    // Block subtree in V1 fetches at depth=2 (focus + 1 hop) so
    // children of the root block are visible.
    expect(call.depth).toBe(2)
  })

  it('returns to the "All" view when the "All" button is clicked again', () => {
    render(<GraphViewPage />)

    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('true')

    fireEvent.click(screen.getByRole('radio', { name: 'All' }))
    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('true')
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('false')
  })
})
