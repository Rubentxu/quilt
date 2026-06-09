import { render, screen, waitFor, fireEvent, act, cleanup } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  SearchModal,
  parseQuery,
  blockMatchesFilter,
} from '../SearchModal'
import { api } from '@core/api-client'

// ──── API + router mocks ───────────────────────────────────────────
// Same shape as SearchModal.test.tsx — SearchModal wires BOTH
// `api.searchPages()` and `api.searchBlocks()`, and the router's
// `useNavigate` is stubbed so the test doesn't need a real router.

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

// ──── Test data ────────────────────────────────────────────────────
//
// `BLOCKS_WITH_PROPS` mirrors the shape returned by the FTS endpoint
// (blockId/pageId/pageName/content/snippet/score/properties). Each
// block carries a structured `properties` bag (S1-04) — the modal
// post-filters against this, not against the raw content.
import type { BlockProperty } from '@shared/types/api'

const PAGES = [
  { id: 'p1', name: 'Foo', title: 'Foo Page', journal: false, journalDay: null, createdAt: '2026-01-02T00:00:00Z' },
  { id: 'p2', name: 'Foobar', title: null, journal: false, journalDay: null, createdAt: '2025-12-01T00:00:00Z' },
]

const BLOCKS_WITH_PROPS: Array<{
  blockId: string
  pageId: string
  pageName: string
  content: string
  snippet: string
  score: number
  properties?: BlockProperty[]
}> = [
  {
    blockId: 'b1',
    pageId: 'p1',
    pageName: 'Foo',
    content: 'task one — no property here',
    snippet: 'task one',
    score: -1.0,
    // No properties bag → filter must reject (S1-04 contract).
  },
  {
    blockId: 'b2',
    pageId: 'p1',
    pageName: 'Foo',
    content: 'task two (content-only, properties present)',
    snippet: 'task two',
    score: -0.5,
    properties: [
      { key: 'status', value: 'done', type: 'string' },
      { key: 'priority', value: 'B', type: 'string' },
    ],
  },
  {
    blockId: 'b3',
    pageId: 'p1',
    pageName: 'Foo',
    content: 'task three (content-only, properties present)',
    snippet: 'task three',
    score: -0.2,
    properties: [
      { key: 'status', value: 'todo', type: 'string' },
      { key: 'priority', value: 'A', type: 'string' },
    ],
  },
]

function renderModal() {
  return render(<SearchModal isOpen={true} onClose={vi.fn()} />)
}

// ──── parseQuery — pure-function tests ─────────────────────────────

describe('parseQuery', () => {
  it('returns empty text and no filters for an empty query', () => {
    expect(parseQuery('')).toEqual({ text: '', filters: [] })
  })

  it('returns empty text and no filters for whitespace-only input', () => {
    expect(parseQuery('   ')).toEqual({ text: '', filters: [] })
  })

  it('treats a known key:value as a filter and removes it from text', () => {
    expect(parseQuery('status:todo')).toEqual({
      text: '',
      filters: [{ key: 'status', value: 'todo' }],
    })
  })

  it('preserves free text and separates filters from it', () => {
    expect(parseQuery('foo bar status:todo')).toEqual({
      text: 'foo bar',
      filters: [{ key: 'status', value: 'todo' }],
    })
  })

  it('parses multiple filters in the same query', () => {
    expect(parseQuery('status:todo priority:A created_by:claude')).toEqual({
      text: '',
      filters: [
        { key: 'status', value: 'todo' },
        { key: 'priority', value: 'A' },
        { key: 'created_by', value: 'claude' },
      ],
    })
  })

  it('mixes text and multiple filters while preserving token order', () => {
    const r = parseQuery('hello status:todo world priority:A')
    expect(r.text).toBe('hello world')
    expect(r.filters).toEqual([
      { key: 'status', value: 'todo' },
      { key: 'priority', value: 'A' },
    ])
  })

  it('supports both card-shape and card_shape as filter keys', () => {
    expect(parseQuery('card-shape:reference')).toEqual({
      text: '',
      filters: [{ key: 'card-shape', value: 'reference' }],
    })
    expect(parseQuery('card_shape:content')).toEqual({
      text: '',
      filters: [{ key: 'card_shape', value: 'content' }],
    })
  })

  it('lowercases the filter key on output', () => {
    expect(parseQuery('STATUS:todo')).toEqual({
      text: '',
      filters: [{ key: 'status', value: 'todo' }],
    })
  })

  it('does NOT treat unknown keys as filters (stays in text)', () => {
    // "author" is not in SUPPORTED_FILTER_KEYS — falls through to text
    expect(parseQuery('author:foo bar')).toEqual({
      text: 'author:foo bar',
      filters: [],
    })
  })

  it('does NOT treat a key without a value (status:) as a filter', () => {
    // Regex requires at least one char after the colon
    expect(parseQuery('status: foo')).toEqual({
      text: 'status: foo',
      filters: [],
    })
  })

  it('keeps the value verbatim — including hyphens and underscores', () => {
    expect(parseQuery('created_by:agent::claude')).toEqual({
      text: '',
      filters: [{ key: 'created_by', value: 'agent::claude' }],
    })
  })
})

// ──── blockMatchesFilter — pure-function tests (S1-04) ─────────────
//
// The contract changed: `blockMatchesFilter` no longer regex-matches
// the block's raw content. It looks up the filter key in the block's
// structured `properties` bag. These tests assert the NEW contract —
// they would fail against the old (content-based) implementation,
// which is exactly the regression S1-04 closes.

describe('blockMatchesFilter', () => {
  it('matches when the block has a string property with the same value', () => {
    const block = {
      properties: [{ key: 'status', value: 'todo', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(true)
  })

  it('is case-insensitive on the key', () => {
    const block = {
      properties: [{ key: 'Status', value: 'todo', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(true)
  })

  it('is case-insensitive on the value', () => {
    const block = {
      properties: [{ key: 'status', value: 'TODO', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(true)
  })

  it('returns false when the property is not present', () => {
    const block = { properties: [] as BlockProperty[] }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(false)
  })

  it('returns false when the value does not match', () => {
    const block = {
      properties: [{ key: 'status', value: 'done', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(false)
  })

  it('does not match partial key names (substatus is not status)', () => {
    const block = {
      properties: [{ key: 'substatus', value: 'todo', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(false)
  })

  it('returns false when properties is undefined', () => {
    expect(blockMatchesFilter({}, { key: 'status', value: 'todo' })).toBe(false)
  })

  it('returns false when properties is an empty array', () => {
    expect(
      blockMatchesFilter({ properties: [] }, { key: 'status', value: 'todo' }),
    ).toBe(false)
  })

  it('coerces numeric property values to strings for comparison', () => {
    // Numbers on the wire round-trip via String() coercion — a user
    // typing `priority:42` should match a property whose stored value
    // is the number 42.
    const block = {
      properties: [{ key: 'priority', value: 42, type: 'number' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'priority', value: '42' })).toBe(true)
  })

  it('coerces boolean property values to strings for comparison', () => {
    const block = {
      properties: [{ key: 'done', value: true, type: 'boolean' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'done', value: 'true' })).toBe(true)
    expect(blockMatchesFilter(block, { key: 'done', value: 'false' })).toBe(false)
  })

  // ── S1-04 regression test ────────────────────────────────────────
  // The original bug: a block whose body says "we discussed the
  // status: idea" used to satisfy a `status:todo` filter (regex on
  // content). The new contract requires structured properties, so
  // that body no longer matches. This test pins the fix.
  it('S1-04 regression: a body string that mentions the key but has no matching property does NOT match', () => {
    const block = {
      // No structured `status` property — just body text that
      // happens to say "status:" in passing.
      properties: [{ key: 'topic', value: 'status', type: 'string' }] as BlockProperty[],
    }
    expect(blockMatchesFilter(block, { key: 'status', value: 'todo' })).toBe(false)
  })
})

// ──── UI behavior — chip display, post-filter, editable input ─────

describe('SearchModal — filter UI', () => {
  it('input is directly editable (not read-only)', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(
      /Search pages and blocks/i,
    ) as HTMLInputElement

    // No readOnly attribute on the rendered input.
    expect(input.readOnly).toBe(false)
    // Typing actually updates the value.
    await userEvent.type(input, 'hello')
    expect(input.value).toBe('hello')
  })

  it('shows a filter chip when the user types a filter', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'status:todo')

    // The chip row appears with one chip.
    await waitFor(() => {
      expect(screen.getByTestId('filter-chips')).toBeInTheDocument()
    })
    expect(screen.getByTestId('filter-chip-status-0')).toBeInTheDocument()
    expect(screen.getByText('status:todo')).toBeInTheDocument()
  })

  it('does NOT show the chip row when the query has no filters', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'hello world')

    // The chip row is absent.
    expect(screen.queryByTestId('filter-chips')).not.toBeInTheDocument()
  })

  it('post-filters FTS results by the status property (AND semantics)', async () => {
    mockSearchPages.mockResolvedValue([])
    // FTS returns ALL three blocks; the post-filter must narrow to b3.
    mockSearchBlocks.mockResolvedValue(BLOCKS_WITH_PROPS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'task status:todo')

    // FTS is called with the text part only (not the filter).
    await waitFor(() => {
      expect(mockSearchBlocks).toHaveBeenCalledWith('task', 10)
    })

    // b3 (status:: todo) is visible; b1 and b2 are not.
    await waitFor(() => {
      expect(screen.getByText('task three')).toBeInTheDocument()
    })
    expect(screen.queryByText('task one')).not.toBeInTheDocument()
    expect(screen.queryByText('task two')).not.toBeInTheDocument()
  })

  it('combines multiple filters with AND semantics', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS_WITH_PROPS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    // Both filters must match — only b3 has both.
    await userEvent.type(input, 'task status:todo priority:A')

    await waitFor(() => {
      expect(screen.getByText('task three')).toBeInTheDocument()
    })
    expect(screen.queryByText('task one')).not.toBeInTheDocument()
    expect(screen.queryByText('task two')).not.toBeInTheDocument()

    // Both chips visible.
    expect(screen.getByTestId('filter-chip-status-0')).toBeInTheDocument()
    expect(screen.getByTestId('filter-chip-priority-1')).toBeInTheDocument()
  })

  it('falls back to the first filter value as the FTS query when only filters are typed', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    // No free text — parsed.text is empty, so FTS gets the filter's value.
    await userEvent.type(input, 'status:todo')

    await waitFor(() => {
      expect(mockSearchBlocks).toHaveBeenCalledWith('todo', 10)
    })
  })

  it('removes a filter when the chip X button is clicked', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(
      /Search pages and blocks/i,
    ) as HTMLInputElement

    // Type a query with one filter + free text.
    await userEvent.type(input, 'foo status:todo')
    await waitFor(() => {
      expect(screen.getByTestId('filter-chip-status-0')).toBeInTheDocument()
    })

    // Click the chip's remove button (targeted by aria-label).
    const removeBtn = screen.getByLabelText(/Remove filter status:todo/i)
    await userEvent.click(removeBtn)

    // The chip row is gone.
    await waitFor(() => {
      expect(screen.queryByTestId('filter-chips')).not.toBeInTheDocument()
    })

    // The input is back to just the free-text part.
    expect(input.value).toBe('foo')
  })

  it('removing one of several filters keeps the others', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(
      /Search pages and blocks/i,
    ) as HTMLInputElement

    await userEvent.type(input, 'foo status:todo priority:A')
    await waitFor(() => {
      expect(screen.getByTestId('filter-chip-status-0')).toBeInTheDocument()
      expect(screen.getByTestId('filter-chip-priority-1')).toBeInTheDocument()
    })

    // Remove the status filter; priority should survive.
    const removeStatus = screen.getByLabelText(/Remove filter status:todo/i)
    await userEvent.click(removeStatus)

    await waitFor(() => {
      expect(screen.queryByTestId('filter-chip-status-0')).not.toBeInTheDocument()
    })
    expect(screen.getByTestId('filter-chip-priority-0')).toBeInTheDocument()
    expect(input.value).toBe('foo priority:A')
  })

  it('does not close the modal when the chip X is clicked', async () => {
    const onClose = vi.fn()
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    render(<SearchModal isOpen={true} onClose={onClose} />)
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'status:todo')

    await waitFor(() => {
      expect(screen.getByTestId('filter-chip-status-0')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByLabelText(/Remove filter status:todo/i))

    // onClose was NOT called — only the X on the modal closes it.
    expect(onClose).not.toHaveBeenCalled()
  })

  it('shows the filter chip with the correct key and value', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'priority:B')

    await waitFor(() => {
      expect(screen.getByTestId('filter-chip-priority-0')).toBeInTheDocument()
    })
    // The chip text combines key and value.
    const chip = screen.getByTestId('filter-chip-priority-0')
    expect(chip).toHaveTextContent('priority:B')
  })
})

// ──── Page ranking ────────────────────────────────────────────────

describe('SearchModal — page result ranking', () => {
  it('sorts page results by createdAt descending (most recent first)', async () => {
    mockSearchPages.mockResolvedValue(PAGES) // p1 = 2026-01-02, p2 = 2025-12-01
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    // Query matches both pages.
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('Foo Page')).toBeInTheDocument()
    })

    // p1 (newer) should appear before p2 (older) in the list.
    const buttons = screen.getAllByRole('button')
    const idxFoo = buttons.findIndex(b => b.textContent?.includes('Foo Page'))
    const idxFoobar = buttons.findIndex(b => b.textContent?.includes('Foobar'))
    expect(idxFoo).toBeLessThan(idxFoobar)
    expect(idxFoo).toBeGreaterThan(-1)
    expect(idxFoobar).toBeGreaterThan(-1)
  })
})

// ──── Debounce (property of the wiring, not a new feature) ────────

describe('SearchModal — debounce interacts correctly with filter parsing', () => {
  it('sends the final parsed text (without filter tokens) to FTS after debounce settles', async () => {
    vi.useFakeTimers()
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)

    // Type text + filter rapidly; debounce should coalesce.
    await act(async () => {
      fireEvent.change(input, { target: { value: 's' } })
      fireEvent.change(input, { target: { value: 'st' } })
      fireEvent.change(input, { target: { value: 'sta' } })
      fireEvent.change(input, { target: { value: 'stat' } })
      fireEvent.change(input, { target: { value: 'status' } })
      fireEvent.change(input, { target: { value: 'status:t' } })
      fireEvent.change(input, { target: { value: 'status:to' } })
      fireEvent.change(input, { target: { value: 'status:tod' } })
      fireEvent.change(input, { target: { value: 'status:todo' } })
    })

    // Pre-debounce: FTS untouched.
    expect(mockSearchBlocks).not.toHaveBeenCalled()

    await act(async () => {
      await vi.advanceTimersByTimeAsync(250)
    })

    // Filter-only query → FTS gets the filter's value, not "status:todo".
    expect(mockSearchBlocks).toHaveBeenCalledTimes(1)
    expect(mockSearchBlocks).toHaveBeenCalledWith('todo', 10)
  })
})
