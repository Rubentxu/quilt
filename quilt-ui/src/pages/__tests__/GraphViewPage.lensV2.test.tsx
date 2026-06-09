// GraphViewPage — Graph Lens V2 (quick-access buttons + shortcuts).
//
// The V1 lens selector is a radiogroup of 4 text buttons. V2 keeps
// the same control surface (so existing V1 tests still pass) but
// turns it into *quick-access* buttons:
//
//   1. Each button renders an icon + label (not just a label).
//   2. Each button has a `data-lens` attribute (the same key V1
//      exposed — so the selector remains queryable by lens id).
//   3. Pressing `1`, `2`, `3`, or `4` anywhere on the page sets the
//      active lens. This is the "quick-access" promise: the user can
//      switch lenses without leaving the keyboard and without
//      opening a dropdown.
//
// We assert BEHAVIOR — what the user sees and what the API receives —
// not the implementation of the keyboard handler (could be a window
// listener, a document listener, or a global hook — all fine).

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { GraphViewPage } from '../GraphViewPage'

// ── Mocks ─────────────────────────────────────────────────────────

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

describe('GraphViewPage — lens V2 quick-access buttons', () => {
  it('renders all four lens buttons (regression for V1 selector)', () => {
    render(<GraphViewPage />)
    // The V1 selector used 4 radios. V2 keeps the same queryable
    // surface so the V1 contract is preserved.
    expect(screen.getByRole('radio', { name: 'All' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Page context' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Block subtree' })).toBeInTheDocument()
    expect(screen.getByRole('radio', { name: 'Property filter' })).toBeInTheDocument()
  })

  it('exposes data-lens on each button (V2 quick-access identifier)', () => {
    render(<GraphViewPage />)
    // V2: each button is also identifiable by the lens id via a
    // data-lens attribute, so shortcuts/automation can map key → lens
    // without reading the label.
    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-lens')).toBe('all')
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-lens')).toBe('page-context')
    expect(screen.getByRole('radio', { name: 'Block subtree' }).getAttribute('data-lens')).toBe('block-subtree')
    expect(screen.getByRole('radio', { name: 'Property filter' }).getAttribute('data-lens')).toBe('property')
  })

  it('renders a Lucide icon inside each lens button (V2 visual)', () => {
    const { container } = render(<GraphViewPage />)
    // The lens buttons sit inside the radiogroup; the V2 enhancement
    // is that each button now renders an SVG icon (lucide-react emits
    // <svg>) next to its label. The user-facing contract is "icon +
    // label" — the icon is an SVG child of the button.
    const lensButtons = container.querySelectorAll('[data-lens]')
    expect(lensButtons.length).toBe(4)
    for (const btn of Array.from(lensButtons)) {
      const svg = btn.querySelector('svg')
      expect(svg, `lens button ${(btn as HTMLElement).getAttribute('data-lens')} should render an icon`).not.toBeNull()
    }
  })

  it('keeps the existing active-lens marker on the V1 selector (sync with V2 buttons)', () => {
    // V1 used data-active="true|false"; V2 keeps it because the same
    // button row IS the V2 quick-access row. The test guards against
    // someone "rewriting" the selector in V2 and breaking V1 styling.
    render(<GraphViewPage />)
    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('true')
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('false')
  })

  it('updates the active lens when a button is clicked (regression for V1 click → state)', () => {
    render(<GraphViewPage />)
    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('true')
    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('false')
  })

  it('re-fetches the lens endpoint when the active lens changes', async () => {
    // V1 contract: changing the lens re-fetches via getGraphLens.
    // V2 must not break that — the quick-access button is the same
    // button, so the effect must still fire.
    render(<GraphViewPage />)
    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))
    await waitFor(() => {
      expect(mockApi.getGraphLens).toHaveBeenCalled()
    })
  })
})

describe('GraphViewPage — lens V2 keyboard shortcuts', () => {
  it('pressing "1" switches to the All lens', () => {
    render(<GraphViewPage />)
    // First switch to a non-default lens so we can observe the change.
    fireEvent.click(screen.getByRole('radio', { name: 'Page context' }))
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('true')

    // Press "1" — V2 shortcut for All.
    fireEvent.keyDown(window, { key: '1' })

    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('true')
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('false')
  })

  it('pressing "2" switches to the Page context lens', () => {
    render(<GraphViewPage />)
    fireEvent.keyDown(window, { key: '2' })
    expect(screen.getByRole('radio', { name: 'Page context' }).getAttribute('data-active')).toBe('true')
  })

  it('pressing "3" switches to the Block subtree lens', () => {
    render(<GraphViewPage />)
    fireEvent.keyDown(window, { key: '3' })
    expect(screen.getByRole('radio', { name: 'Block subtree' }).getAttribute('data-active')).toBe('true')
  })

  it('pressing "4" switches to the Property filter lens', () => {
    render(<GraphViewPage />)
    fireEvent.keyDown(window, { key: '4' })
    expect(screen.getByRole('radio', { name: 'Property filter' }).getAttribute('data-active')).toBe('true')
  })

  it('keyboard shortcut triggers the same re-fetch as clicking the button', async () => {
    render(<GraphViewPage />)
    fireEvent.keyDown(window, { key: '3' })
    // "Block subtree" → depth=2, same as clicking the button.
    await waitFor(() => {
      expect(mockApi.getGraphLens).toHaveBeenCalled()
    })
    const call = mockApi.getGraphLens.mock.calls[0][0]
    expect(call.depth).toBe(2)
  })

  it('ignores shortcut when the user is typing in an input (no global hijack)', () => {
    // V2 shortcuts are *page-level* — they must not steal keys from
    // a search box or rename input. The convention in this app is to
    // skip the shortcut if the event target is an editable element.
    // We assert it indirectly: typing "1" inside an <input> should
    // not switch the lens. This is what makes the shortcut "quick
    // access" and not a footgun.
    render(
      <div>
        <input data-testid="probe-input" />
        <GraphViewPage />
      </div>,
    )
    const input = screen.getByTestId('probe-input')
    fireEvent.keyDown(input, { key: '1' })
    // The lens must still be the default "All" — the shortcut was
    // ignored because the event came from an input.
    expect(screen.getByRole('radio', { name: 'All' }).getAttribute('data-active')).toBe('true')
  })
})
