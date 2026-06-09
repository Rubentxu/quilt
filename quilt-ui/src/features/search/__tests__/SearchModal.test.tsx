import { render, screen, waitFor, fireEvent, act, cleanup } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { SearchModal, FOCUS_BLOCK_STORAGE_KEY } from '../SearchModal'
import { api } from '@core/api-client'

// ──── API + router mocks ───────────────────────────────────────────
// SearchModal wires BOTH `api.searchPages()` (server-side page-name
// filter — S2-03) and `api.searchBlocks()` (FTS over block content —
// G3 of the wikilinks audit). The router's `useNavigate` is stubbed
// so the test doesn't need a real router instance.

const mockSearchPages = vi.fn()
const mockSearchBlocks = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    searchPages: (...args: unknown[]) => mockSearchPages(...args),
    searchBlocks: (...args: unknown[]) => mockSearchBlocks(...args),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

beforeEach(() => {
  mockSearchPages.mockReset()
  mockSearchBlocks.mockReset()
  mockNavigate.mockReset()
  sessionStorage.clear()
})

afterEach(() => {
  cleanup()
  sessionStorage.clear()
  vi.useRealTimers()
})

const PAGES = [
  { id: 'p1', name: 'Foo', title: 'Foo Page', journal: false, journalDay: null, createdAt: '' },
  { id: 'p2', name: 'Foobar', title: null, journal: false, journalDay: null, createdAt: '' },
  { id: 'p3', name: 'Other', title: 'Other Page', journal: false, journalDay: null, createdAt: '' },
]

const BLOCKS = [
  { blockId: 'b1', pageId: 'p2', pageName: 'Foobar', content: 'this is a foo in the wild', snippet: 'this is a foo in the wild', score: -1.0 },
  { blockId: 'b2', pageId: 'p1', pageName: 'Foo', content: 'another block mentioning foo', snippet: 'another block mentioning foo', score: -0.5 },
]

function renderModal() {
  return render(<SearchModal isOpen={true} onClose={vi.fn()} />)
}

describe('SearchModal', () => {
  it('shows pages when the query is empty (no block search call)', async () => {
    mockSearchPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue([])

    renderModal()

    await waitFor(() => {
      expect(screen.getByText('Foo Page')).toBeInTheDocument()
    })
    expect(screen.getByText('Other Page')).toBeInTheDocument()
    // No "Blocks" section when the query is empty.
    expect(screen.queryByText('Blocks')).not.toBeInTheDocument()
    // FTS endpoint is NOT called for an empty query — that would be a
    // round-trip per modal open and the backend would 400 on it.
    expect(mockSearchBlocks).not.toHaveBeenCalled()
  })

  it('shows both page matches and block matches when the query is non-empty', async () => {
    // S2-03: the search modal now drives BOTH the empty-query and
    // the typed-query paths through `api.searchPages()`. The server
    // returns a filtered list when a query is passed, so the
    // `Other` page (whose name doesn't contain "foo") is absent
    // from the typed-query response — no client-side filter
    // required. The empty-query response, on the other hand, is
    // the full PAGES list (mirroring the previous "listPages"
    // behaviour).
    mockSearchPages.mockImplementation(async (q: string) => {
      if (!q) return PAGES
      return PAGES.filter(p => p.name.toLowerCase().includes(q.toLowerCase()))
    })
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)

    await userEvent.type(input, 'foo')

    // Block section: the FTS result snippet/preview.
    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    // Page section: only pages whose name contains 'foo'.
    expect(screen.getByText('Foo Page')).toBeInTheDocument()
    // "Other Page" must NOT be present — the server-side filter
    // (simulated by mockImplementation above) excluded it.
    expect(screen.queryByText('Other Page')).not.toBeInTheDocument()

    // The block's parent page appears as a subtitle so the user knows
    // which page they'd land on. There are two "Foobar" rows visible
    // (the page row, and the block's subtitle), so we use getAllByText.
    expect(screen.getAllByText('Foobar').length).toBeGreaterThanOrEqual(1)

    // Section headers should both be present.
    expect(screen.getByText('Pages')).toBeInTheDocument()
    expect(screen.getByText('Blocks')).toBeInTheDocument()

    // FTS endpoint was called once with the typed query.
    expect(mockSearchBlocks).toHaveBeenCalledWith('foo', 10)
  })

  it('truncates block content previews to ~80 characters', async () => {
    const longContent = 'x'.repeat(200)
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([
      { blockId: 'b1', pageId: 'p1', pageName: 'P', content: longContent, snippet: longContent, score: -1 },
    ])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'x')

    await waitFor(() => {
      // Look for the truncated text: 80 chars + ellipsis.
      const el = screen.getByText(/x{80}…/)
      expect(el).toBeInTheDocument()
    })
  })

  it('debounces the search — typing fast does not fire one request per keystroke', async () => {
    vi.useFakeTimers()
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)

    // Type 5 characters in quick succession.
    await act(async () => {
      fireEvent.change(input, { target: { value: 'f' } })
      fireEvent.change(input, { target: { value: 'fo' } })
      fireEvent.change(input, { target: { value: 'foo' } })
      fireEvent.change(input, { target: { value: 'foob' } })
      fireEvent.change(input, { target: { value: 'fooba' } })
      fireEvent.change(input, { target: { value: 'foobar' } })
    })

    // Before the debounce timer fires, the FTS endpoint has not been
    // called.
    expect(mockSearchBlocks).not.toHaveBeenCalled()

    // Advance just past the debounce delay (200ms in the modal).
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250)
    })

    // Exactly one FTS call, with the final query value.
    expect(mockSearchBlocks).toHaveBeenCalledTimes(1)
    expect(mockSearchBlocks).toHaveBeenCalledWith('foobar', 10)
  })

  it('navigates to a page when a page result is clicked', async () => {
    mockSearchPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('Foo Page')).toBeInTheDocument()
    })
    await userEvent.click(screen.getByText('Foo Page'))

    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/page/$name',
      params: { name: 'Foo' },
    })
    // Block focus storage is untouched — only set for block results.
    expect(sessionStorage.getItem(FOCUS_BLOCK_STORAGE_KEY)).toBeNull()
  })

  it('navigates to a block result and stores the focus request', async () => {
    mockSearchPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })
    await userEvent.click(screen.getByText('this is a foo in the wild'))

    // PageView reads FOCUS_BLOCK_STORAGE_KEY on mount to focus the block.
    expect(sessionStorage.getItem(FOCUS_BLOCK_STORAGE_KEY)).toBe('b1')
    // And we navigate to the block's parent page.
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/page/$name',
      params: { name: 'Foobar' },
    })
  })

  it('ArrowDown / ArrowUp move the selection through the unified list', async () => {
    // S2-03: the server-side filter strips the non-matching 'Other'
    // page from the typed-query response. The flat result list is
    // [Foo, Foobar, block1, block2], so two ArrowDowns land on the
    // first block.
    mockSearchPages.mockImplementation(async (q: string) => {
      if (!q) return PAGES
      return PAGES.filter(p => p.name.toLowerCase().includes(q.toLowerCase()))
    })
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    // Default: index 0 (the first page match — "Foo").
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    // Now on index 1 (the second page match — "Foobar").
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    // Now on the first block (index 2 in the flat list).
    fireEvent.keyDown(input, { key: 'Enter' })

    // Enter on a block result focuses the block and navigates.
    expect(sessionStorage.getItem(FOCUS_BLOCK_STORAGE_KEY)).toBe('b1')
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/page/$name',
      params: { name: 'Foobar' },
    })
  })
})
