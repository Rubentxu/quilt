// SearchModal — Saved/Recent searches (ROADMAP #22)
//
// These tests cover the UI integration of the recent + saved
// searches feature on top of the SearchModal. The pure persistence
// logic (FIFO eviction, defensive parse, dedupe) is covered in
// `savedSearches.test.ts`; this file is concerned with what the user
// SEES and what happens when they click.
//
// Behaviour matrix:
//   - Input empty: show "Recent" + "Saved" lists instead of the
//     "Pages" / "Blocks" sections.
//   - Input non-empty: show the normal search results, with a "Save
//     search" button when results are present.
//   - Click recent search → re-executes it (puts it in the input).
//   - Click saved search → re-executes the saved DSL.
//   - Save form creates a saved search and persists it.
//   - Delete saved search works (× button per row).
//   - Recent searches are auto-saved when a search has run (modal
//     closes with a result set) — see "auto-save on close" test.

import { render, screen, waitFor, fireEvent, cleanup, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { SearchModal } from '../SearchModal'
import {
  RECENT_SEARCHES_KEY,
  SAVED_SEARCHES_KEY,
} from '../savedSearches'
import type { SavedSearch } from '../savedSearches'

// ──── API + router mocks ───────────────────────────────────────────

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
  localStorage.clear()
})

afterEach(() => {
  cleanup()
  sessionStorage.clear()
  localStorage.clear()
  vi.useRealTimers()
})

// Helper — render with isOpen=true and a configurable onClose spy.
function renderModal(overrides: { isOpen?: boolean; onClose?: () => void } = {}) {
  const onClose = overrides.onClose ?? vi.fn()
  const utils = render(
    <SearchModal isOpen={overrides.isOpen ?? true} onClose={onClose} />,
  )
  return { ...utils, onClose }
}

const PAGES = [
  { id: 'p1', name: 'Foo', title: 'Foo Page', journal: false, journalDay: null, createdAt: '2026-01-02T00:00:00Z' },
]

const BLOCKS = [
  { blockId: 'b1', pageId: 'p1', pageName: 'Foo', content: 'foo content', snippet: 'foo content', score: -1 },
]

function getInput() {
  return screen.getByPlaceholderText(/Search pages and blocks/i) as HTMLInputElement
}

// ──── 1. Recent searches show on empty input ───────────────────────

describe('SearchModal — recent searches panel', () => {
  it('does NOT show recent searches when the history is empty', async () => {
    mockSearchPages.mockResolvedValue(PAGES)
    renderModal()

    // The default "Pages" section appears (from searchPages), but the
    // "Recent searches" header is absent because nothing is stored.
    await waitFor(() => {
      expect(screen.getByText('Foo Page')).toBeInTheDocument()
    })
    expect(screen.queryByText(/recent/i)).not.toBeInTheDocument()
  })

  it('shows the recent searches list when the input is empty and the history is non-empty', async () => {
    localStorage.setItem(
      RECENT_SEARCHES_KEY,
      JSON.stringify([
        { query: 'alpha', timestamp: 100, resultCount: 3 },
        { query: 'beta', timestamp: 200, resultCount: 1 },
      ]),
    )
    mockSearchPages.mockResolvedValue([])
    renderModal()

    await waitFor(() => {
      expect(screen.getByText('alpha')).toBeInTheDocument()
    })
    expect(screen.getByText('beta')).toBeInTheDocument()
    expect(screen.getByText(/recent searches/i)).toBeInTheDocument()
  })

  it('hides the recent searches list while the user is typing (search results take over)', async () => {
    localStorage.setItem(
      RECENT_SEARCHES_KEY,
      JSON.stringify([{ query: 'alpha', timestamp: 1, resultCount: 0 }]),
    )
    mockSearchPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('foo content')).toBeInTheDocument()
    })
    // The recent-searches header should be gone now that input is non-empty.
    expect(screen.queryByText(/recent searches/i)).not.toBeInTheDocument()
  })
})

// ──── 2. Click recent search re-executes it ────────────────────────

describe('SearchModal — clicking a recent search', () => {
  it('re-executes the search by putting the query back into the input', async () => {
    localStorage.setItem(
      RECENT_SEARCHES_KEY,
      JSON.stringify([{ query: 'alpha', timestamp: 1, resultCount: 0 }]),
    )
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([
      { blockId: 'b1', pageId: 'p1', pageName: 'P', content: 'alpha hit', snippet: 'alpha hit', score: -1 },
    ])

    renderModal()
    const recentBtn = await screen.findByText('alpha')
    await userEvent.click(recentBtn)

    // The input is now 'alpha'.
    const input = getInput()
    expect(input.value).toBe('alpha')

    // The search has been re-issued: FTS is called with 'alpha'.
    await waitFor(() => {
      expect(mockSearchBlocks).toHaveBeenCalledWith('alpha', 10)
    })
  })
})

// ──── 3. Auto-save to recents after a search ──────────────────────

describe('SearchModal — auto-save recent searches', () => {
  it('writes the executed query to localStorage when the user has typed and the modal closes', async () => {
    mockSearchPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    const onClose = vi.fn()
    const { rerender } = render(<SearchModal isOpen={true} onClose={onClose} />)
    const input = getInput()
    await userEvent.type(input, 'foo')

    // Wait for FTS to be called so we know the search "happened".
    await waitFor(() => {
      expect(mockSearchBlocks).toHaveBeenCalledWith('foo', 10)
    })

    // Close the modal. The close-effect that writes to recents runs
    // in the same effect flush, so we wrap the rerender in act() to
    // guarantee the flush before we read localStorage.
    act(() => {
      rerender(<SearchModal isOpen={false} onClose={onClose} />)
    })

    // localStorage now has the query. The effect runs synchronously
    // in the same flush as the rerender, but we still waitFor to be
    // defensive against any micro-task ordering quirks in the test
    // environment (the prior implementation was occasionally flaky
    // here, so we belt-and-brace it).
    await waitFor(() => {
      const raw = localStorage.getItem(RECENT_SEARCHES_KEY)
      expect(raw).not.toBeNull()
      const parsed = JSON.parse(raw!) as Array<{ query: string }>
      expect(parsed[0].query).toBe('foo')
      expect(parsed[0].resultCount).toBeGreaterThanOrEqual(0)
    })
  })

  it('caps the recent-search history at 10 entries (FIFO eviction)', async () => {
    // Pre-seed 10 entries (the cap); adding one more must evict the
    // tail (the lowest-priority entry in the prepended/newest-first
    // list). The recents list is sorted newest-first, so eviction
    // drops the last element of the prepended list, which is the
    // oldest seeded entry BY TIMESTAMP — that's `seed-9` in this
    // seed shape (timestamp 1000+i means i=9 is the newest).
    const seeded = Array.from({ length: 10 }, (_, i) => ({
      query: `seed-${i}`,
      timestamp: 1000 + i,
      resultCount: 0,
    }))
    localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(seeded))

    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    const onClose = vi.fn()
    const { rerender } = render(<SearchModal isOpen={true} onClose={onClose} />)
    const input = getInput()
    await userEvent.type(input, 'newquery')

    await waitFor(() => {
      expect(mockSearchBlocks).toHaveBeenCalledWith('newquery', 10)
    })

    act(() => {
      rerender(<SearchModal isOpen={false} onClose={onClose} />)
    })

    await waitFor(() => {
      const raw = localStorage.getItem(RECENT_SEARCHES_KEY)
      expect(raw).not.toBeNull()
      const parsed = JSON.parse(raw!) as Array<{ query: string }>
      expect(parsed).toHaveLength(10)
      expect(parsed[0].query).toBe('newquery')
      // FIFO eviction drops the oldest entry in the seeded set.
      // The recents module's `recordRecentSearch` prepends, so the
      // oldest of the input list (`seed-9`, last in the array) is
      // sliced off when the cap is exceeded.
      expect(parsed.find(p => p.query === 'seed-9')).toBeUndefined()
      // The other seeded entries survive.
      expect(parsed.find(p => p.query === 'seed-0')).toBeDefined()
    })
  })
})

// ──── 4. Save button + form ────────────────────────────────────────

describe('SearchModal — save search', () => {
  it('does NOT show a save button while the input is empty', () => {
    mockSearchPages.mockResolvedValue([])
    renderModal()
    expect(screen.queryByTestId('save-search-button')).not.toBeInTheDocument()
  })

  it('shows a save button after the user has executed a search', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByTestId('save-search-button')).toBeInTheDocument()
    })
  })

  it('opens a save form when the save button is clicked', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')
    await waitFor(() => {
      expect(screen.getByTestId('save-search-button')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByTestId('save-search-button'))

    // The form fields are now present.
    expect(screen.getByTestId('save-search-name-input')).toBeInTheDocument()
    expect(screen.getByTestId('save-search-viewtype-select')).toBeInTheDocument()
    expect(screen.getByTestId('save-search-confirm')).toBeInTheDocument()
  })

  it('saves a new search to localStorage when the form is confirmed', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')
    await waitFor(() => {
      expect(screen.getByTestId('save-search-button')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByTestId('save-search-button'))
    const nameInput = screen.getByTestId('save-search-name-input') as HTMLInputElement
    await userEvent.type(nameInput, 'Open TODOs')
    await userEvent.click(screen.getByTestId('save-search-confirm'))

    const raw = localStorage.getItem(SAVED_SEARCHES_KEY)
    expect(raw).not.toBeNull()
    const parsed = JSON.parse(raw!) as SavedSearch[]
    expect(parsed).toHaveLength(1)
    expect(parsed[0]).toMatchObject({
      name: 'Open TODOs',
      query: 'foo',
    })
    expect(typeof parsed[0].id).toBe('string')
    expect(parsed[0].id.length).toBeGreaterThan(0)
    expect(typeof parsed[0].createdAt).toBe('number')
  })

  it('saves the chosen viewType alongside the search', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')
    await waitFor(() => {
      expect(screen.getByTestId('save-search-button')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByTestId('save-search-button'))
    const nameInput = screen.getByTestId('save-search-name-input') as HTMLInputElement
    await userEvent.type(nameInput, 'K')
    fireEvent.change(screen.getByTestId('save-search-viewtype-select'), {
      target: { value: 'kanban' },
    })
    await userEvent.click(screen.getByTestId('save-search-confirm'))

    const raw = localStorage.getItem(SAVED_SEARCHES_KEY)
    const parsed = JSON.parse(raw!) as SavedSearch[]
    expect(parsed[0].viewType).toBe('kanban')
  })

  it('refuses to save when the name is empty', async () => {
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = getInput()
    await userEvent.type(input, 'foo')
    await waitFor(() => {
      expect(screen.getByTestId('save-search-button')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByTestId('save-search-button'))
    // Type nothing into the name field.
    await userEvent.click(screen.getByTestId('save-search-confirm'))

    // Nothing got persisted.
    expect(localStorage.getItem(SAVED_SEARCHES_KEY)).toBeNull()
  })
})

// ──── 5. Saved searches panel + click re-executes ─────────────────

describe('SearchModal — saved searches panel', () => {
  it('does NOT show the saved section when the store is empty', async () => {
    mockSearchPages.mockResolvedValue([])
    renderModal()
    expect(screen.queryByText(/saved searches/i)).not.toBeInTheDocument()
  })

  it('lists saved searches when the input is empty', async () => {
    const saved: SavedSearch[] = [
      { id: 's1', name: 'Open TODOs', query: 'status:todo', createdAt: 1 },
      { id: 's2', name: 'Recent Notes', query: 'note', createdAt: 2, viewType: 'cards' },
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(saved))
    mockSearchPages.mockResolvedValue([])

    renderModal()

    await waitFor(() => {
      expect(screen.getByText('Open TODOs')).toBeInTheDocument()
    })
    expect(screen.getByText('Recent Notes')).toBeInTheDocument()
    expect(screen.getByText(/saved searches/i)).toBeInTheDocument()
  })

  it('clicking a saved search re-executes it (puts the saved DSL into the input)', async () => {
    const saved: SavedSearch[] = [
      { id: 's1', name: 'Open TODOs', query: 'status:todo', createdAt: 1 },
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(saved))
    mockSearchPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    renderModal()
    const savedBtn = await screen.findByText('Open TODOs')
    await userEvent.click(savedBtn)

    const input = getInput()
    expect(input.value).toBe('status:todo')
  })

  it('persists saved searches across modal sessions (localStorage round-trip)', async () => {
    const saved: SavedSearch[] = [
      { id: 's1', name: 'Open TODOs', query: 'status:todo', createdAt: 1 },
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(saved))
    mockSearchPages.mockResolvedValue([])

    // First mount — the saved search is visible.
    const first = renderModal()
    await waitFor(() => {
      expect(screen.getByText('Open TODOs')).toBeInTheDocument()
    })
    first.unmount()

    // Second mount — same data, same visibility.
    renderModal()
    await waitFor(() => {
      expect(screen.getByText('Open TODOs')).toBeInTheDocument()
    })
  })

  it('deletes a saved search when its × button is clicked', async () => {
    const saved: SavedSearch[] = [
      { id: 's1', name: 'A', query: 'a', createdAt: 1 },
      { id: 's2', name: 'B', query: 'b', createdAt: 2 },
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(saved))
    mockSearchPages.mockResolvedValue([])

    renderModal()
    await waitFor(() => {
      expect(screen.getByText('A')).toBeInTheDocument()
    })

    await userEvent.click(screen.getByTestId('delete-saved-search-s1'))

    const raw = localStorage.getItem(SAVED_SEARCHES_KEY)
    const parsed = JSON.parse(raw!) as SavedSearch[]
    expect(parsed.map(s => s.id)).toEqual(['s2'])
  })
})
