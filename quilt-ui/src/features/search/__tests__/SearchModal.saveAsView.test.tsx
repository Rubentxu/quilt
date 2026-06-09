/**
 * Pure-function + component tests for the "Save as View" flow
 * (ROADMAP #25 — "Save as View" desde search).
 *
 * The flow has three moving parts:
 *
 *   1. The user clicks "Save as View" on a search result (block or
 *      page). The action passes the current query and the chosen
 *      result kind to a small modal.
 *
 *   2. The modal collects `view-name`, `view-type`, and the
 *      `pageName` of the page the view should be created in.
 *      Submitting returns a `SaveAsViewRequest`.
 *
 *   3. The submit handler in SearchModal:
 *        a. Builds the search DSL from the parsed query
 *           (free-text part + filters rewritten as `key:: value`
 *           property syntax).
 *        b. Creates a `type:: query` block in the target page
 *           carrying the DSL.
 *        c. Reads back the new block's id from the response and
 *           creates a `type:: view` block referencing it via
 *           `data-source::`.
 *
 * Tests below pin each step.
 */

import { render, screen, waitFor, fireEvent, act, cleanup } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { SearchModal, buildSearchDsl } from '../SearchModal'
import { SaveAsViewModal } from '../SaveAsViewModal'
import { api } from '@core/api-client'
import type { Page, Block } from '@shared/types/api'

// ──── API + router mocks ───────────────────────────────────────────
//
// S2-03: the SearchModal now uses `api.searchPages()` (server-side
// page-name filter) instead of `api.listPages() + client-side
// includes`. We still keep a `listPages` mock here because the
// "Save as View" flow calls it to populate the page-picker in the
// modal — that's an unrelated, full-page-list use case.

const mockListPages = vi.fn()
const mockSearchPages = vi.fn()
const mockSearchBlocks = vi.fn()
const mockCreateBlock = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    listPages: (...args: unknown[]) => mockListPages(...args),
    searchPages: (...args: unknown[]) => mockSearchPages(...args),
    searchBlocks: (...args: unknown[]) => mockSearchBlocks(...args),
    createBlock: (...args: unknown[]) => mockCreateBlock(...args),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

beforeEach(() => {
  mockListPages.mockReset()
  mockSearchPages.mockReset()
  mockSearchBlocks.mockReset()
  mockCreateBlock.mockReset()
  mockNavigate.mockReset()
  // S2-03: the SearchModal now calls `api.searchPages('', PAGE_LIMIT)`
  // on mount (the empty-query case). Without a default resolved
  // value the call returns `undefined` and the modal crashes before
  // any test logic runs. Default to `PAGES` so the test mirrors the
  // "server returns the full page list" behaviour; individual tests
  // override via `mockImplementation` if they need a different
  // typed-query response.
  mockSearchPages.mockResolvedValue(PAGES)
  sessionStorage.clear()
})

afterEach(() => {
  cleanup()
  sessionStorage.clear()
  vi.useRealTimers()
})

// ──── Shared fixtures ─────────────────────────────────────────────

const PAGES = [
  { id: 'p1', name: 'Foo', title: 'Foo Page', journal: false, journalDay: null, createdAt: '2026-01-02T00:00:00Z' },
  { id: 'p2', name: 'Bar', title: 'Bar Page', journal: false, journalDay: null, createdAt: '2025-12-01T00:00:00Z' },
  { id: 'p3', name: 'Baz', title: null, journal: false, journalDay: null, createdAt: '2025-11-15T00:00:00Z' },
]

const BLOCKS = [
  { blockId: 'b1', pageId: 'p2', pageName: 'Bar', content: 'this is a foo in the wild', snippet: 'this is a foo in the wild', score: -1.0 },
  { blockId: 'b2', pageId: 'p1', pageName: 'Foo', content: 'another block mentioning foo', snippet: 'another block mentioning foo', score: -0.5 },
]

/**
 * A separate fixture for the "filter + text" case: the blocks have
 * short text (so the modal's "this is a foo in the wild" string is
 * still findable in the other tests) and ALSO carry a structured
 * `properties` bag (S1-04) so the post-filter keeps them when the
 * user types `foo status:todo`. The old fixture stored the property
 * only in the body text; the new contract requires structured data.
 */
const BLOCKS_WITH_STATUS = [
  {
    blockId: 'b1',
    pageId: 'p2',
    pageName: 'Bar',
    content: 'this is a foo in the wild',
    snippet: 'this is a foo in the wild',
    score: -1.0,
    properties: [{ key: 'status', value: 'todo', type: 'string' }],
  },
  {
    blockId: 'b2',
    pageId: 'p1',
    pageName: 'Foo',
    content: 'another foo',
    snippet: 'another foo',
    score: -0.5,
    properties: [{ key: 'status', value: 'done', type: 'string' }],
  },
]

/** Build a normalized block the way `normalizeBlock` would. */
function makeBlockResponse(overrides: Partial<Block>): Block {
  return {
    id: overrides.id ?? 'new-block',
    pageId: overrides.pageId ?? 'p1',
    pageName: overrides.pageName ?? 'Foo',
    content: overrides.content ?? '',
    blockType: overrides.blockType ?? 'paragraph',
    marker: overrides.marker ?? null,
    priority: overrides.priority ?? null,
    parentId: overrides.parentId ?? null,
    order: overrides.order ?? 1,
    level: overrides.level ?? 0,
    collapsed: overrides.collapsed ?? false,
    properties: overrides.properties ?? [],
    createdAt: overrides.createdAt ?? '2026-06-09T00:00:00Z',
    updatedAt: overrides.updatedAt ?? '2026-06-09T00:00:00Z',
  }
}

// ──── buildSearchDsl (pure function) ──────────────────────────────

describe('buildSearchDsl — pure function', () => {
  it('returns the trimmed text part when there are no filters', () => {
    expect(buildSearchDsl({ text: 'hello world', filters: [] })).toBe('hello world')
  })

  it('returns the DSL with no leading space when text is empty', () => {
    expect(
      buildSearchDsl({ text: '', filters: [{ key: 'status', value: 'todo' }] }),
    ).toBe('status:: todo')
  })

  it('combines text and a single filter with a single space', () => {
    expect(
      buildSearchDsl({
        text: 'task',
        filters: [{ key: 'status', value: 'todo' }],
      }),
    ).toBe('task status:: todo')
  })

  it('joins multiple filters with single spaces after the text', () => {
    expect(
      buildSearchDsl({
        text: 'task',
        filters: [
          { key: 'status', value: 'todo' },
          { key: 'priority', value: 'A' },
        ],
      }),
    ).toBe('task status:: todo priority:: A')
  })

  it('returns an empty string for an empty parsed query', () => {
    expect(buildSearchDsl({ text: '', filters: [] })).toBe('')
  })

  it('preserves filter values that contain special characters verbatim', () => {
    // created_by often contains "agent::claude" — must survive the round trip.
    expect(
      buildSearchDsl({
        text: '',
        filters: [{ key: 'created_by', value: 'agent::claude' }],
      }),
    ).toBe('created_by:: agent::claude')
  })
})

// ──── SaveAsViewModal (presentational) ────────────────────────────

describe('SaveAsViewModal', () => {
  const baseProps = {
    pages: PAGES as Page[],
    isSubmitting: false,
    errorMessage: null as string | null,
  }

  it('renders the view-name input, view-type select, and page select', () => {
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    )

    expect(screen.getByTestId('save-view-name-input')).toBeInTheDocument()
    expect(screen.getByTestId('save-view-type-select')).toBeInTheDocument()
    expect(screen.getByTestId('save-view-page-select')).toBeInTheDocument()
  })

  it('lists every view-type recognised by the SavedView dispatcher', () => {
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    )

    const select = screen.getByTestId('save-view-type-select') as HTMLSelectElement
    const values = Array.from(select.querySelectorAll('option')).map(o => o.value)
    // The set must match the dispatcher in SavedViewBlock.tsx.
    expect(values).toEqual(
      expect.arrayContaining(['table', 'kanban', 'calendar', 'list', 'graph', 'cards', 'timeline']),
    )
    // "table" is the default — verify it is the first option so the
    // most useful choice is pre-selected.
    expect(values[0]).toBe('table')
  })

  it('populates the page selector with all provided pages', () => {
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    )

    const select = screen.getByTestId('save-view-page-select') as HTMLSelectElement
    const labels = Array.from(select.querySelectorAll('option')).map(o => o.textContent)
    expect(labels).toEqual(expect.arrayContaining(['Foo', 'Bar', 'Baz']))
  })

  it('calls onConfirm with the form values when the submit button is clicked', async () => {
    const onConfirm = vi.fn()
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={onConfirm}
        onCancel={vi.fn()}
      />,
    )

    fireEvent.change(screen.getByTestId('save-view-name-input'), {
      target: { value: 'Open tasks' },
    })
    fireEvent.change(screen.getByTestId('save-view-type-select'), {
      target: { value: 'kanban' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Bar' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    expect(onConfirm).toHaveBeenCalledTimes(1)
    expect(onConfirm).toHaveBeenCalledWith({
      name: 'Open tasks',
      viewType: 'kanban',
      pageName: 'Bar',
    })
  })

  it('calls onCancel when the cancel button is clicked', async () => {
    const onCancel = vi.fn()
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={vi.fn()}
        onCancel={onCancel}
      />,
    )

    await userEvent.click(screen.getByTestId('save-view-cancel'))
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('disables the submit button while submitting', () => {
    render(
      <SaveAsViewModal
        {...baseProps}
        isSubmitting={true}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    )

    expect(
      (screen.getByTestId('save-view-submit') as HTMLButtonElement).disabled,
    ).toBe(true)
  })

  it('shows an error message when errorMessage is set', () => {
    render(
      <SaveAsViewModal
        {...baseProps}
        errorMessage="Server exploded"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    )

    const err = screen.getByTestId('save-view-error')
    expect(err).toBeInTheDocument()
    expect(err).toHaveTextContent('Server exploded')
  })

  it('does not call onConfirm when the view-name is empty (the form would be useless)', async () => {
    const onConfirm = vi.fn()
    render(
      <SaveAsViewModal
        {...baseProps}
        onConfirm={onConfirm}
        onCancel={vi.fn()}
      />,
    )

    // Leave the name empty, but pick a page and a type.
    fireEvent.change(screen.getByTestId('save-view-type-select'), {
      target: { value: 'table' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Foo' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    expect(onConfirm).not.toHaveBeenCalled()
  })
})

// ──── SearchModal — Save as View integration ─────────────────────

describe('SearchModal — Save as View integration', () => {
  function renderModal() {
    return render(<SearchModal isOpen={true} onClose={vi.fn()} />)
  }

  it('shows a "Save as View" button on every search result', async () => {
    mockListPages.mockResolvedValue(PAGES)
    // S2-03: server-side filter strips non-matching pages from the
    // typed-query response. The "Bar" and "Baz" pages don't contain
    // "foo" in their name or title, so the server (simulated by
    // this mock) returns only the matching page.
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

    // The query "foo" matches 1 page (only "Foo Page" — Bar and Baz
    // do not contain "foo" in name or title) and both block results
    // (b1 and b2 both contain "foo" in their content) → 1 + 2 = 3
    // save buttons. We count the buttons via a regex so we don't
    // pick up the ESC button at the top of the modal.
    const saveButtons = screen.getAllByTestId(/^save-as-view-/)
    expect(saveButtons.length).toBe(3)
  })

  it('clicking the "Save as View" button on a result opens the save modal', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    // The first result is the first page match — click its save button.
    const firstSave = screen.getAllByTestId(/^save-as-view-/)[0]
    await userEvent.click(firstSave)

    // The modal mounts.
    expect(screen.getByTestId('save-view-name-input')).toBeInTheDocument()
    expect(screen.getByTestId('save-view-type-select')).toBeInTheDocument()
    expect(screen.getByTestId('save-view-page-select')).toBeInTheDocument()
  })

  it('cancelling the modal does NOT create any blocks', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    await userEvent.click(screen.getAllByTestId(/^save-as-view-/)[0])
    await userEvent.click(screen.getByTestId('save-view-cancel'))

    expect(mockCreateBlock).not.toHaveBeenCalled()
  })

  it('submitting the modal creates a type::query block first, then a type::view block referencing it', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)

    // First call (the query block) returns id="q-1", second (the view
    // block) returns id="v-1". This proves the view is wired up to
    // the new query by UUID.
    mockCreateBlock
      .mockResolvedValueOnce(
        makeBlockResponse({ id: 'q-1', content: 'foo', properties: [{ key: 'type', value: 'query', type: 'string' }] }),
      )
      .mockResolvedValueOnce(
        makeBlockResponse({ id: 'v-1', content: '', properties: [{ key: 'type', value: 'view', type: 'string' }] }),
      )

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    // Click "Save as View" on the first result.
    await userEvent.click(screen.getAllByTestId(/^save-as-view-/)[0])

    // Fill the form.
    fireEvent.change(screen.getByTestId('save-view-name-input'), {
      target: { value: 'Foo results' },
    })
    fireEvent.change(screen.getByTestId('save-view-type-select'), {
      target: { value: 'kanban' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Bar' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    // Two createBlock calls in the right order.
    await waitFor(() => {
      expect(mockCreateBlock).toHaveBeenCalledTimes(2)
    })

    // 1. The query block — type::query + dsl:: <the search DSL>
    const [queryCallArgs] = mockCreateBlock.mock.calls[0]
    expect(queryCallArgs).toEqual({
      pageName: 'Bar',
      content: 'foo',
      properties: {
        type: 'query',
        dsl: 'foo',
      },
    })

    // 2. The view block — type::view + view-type + view-name +
    //    data-source pointing at the new query's UUID.
    const [viewCallArgs] = mockCreateBlock.mock.calls[1]
    expect(viewCallArgs).toEqual({
      pageName: 'Bar',
      content: '',
      properties: {
        type: 'view',
        'view-type': 'kanban',
        'view-name': 'Foo results',
        'data-source': 'q-1',
      },
    })
  })

  it('reconstructs the DSL with property filters for the query block', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS_WITH_STATUS)
    mockCreateBlock
      .mockResolvedValueOnce(makeBlockResponse({ id: 'q-1' }))
      .mockResolvedValueOnce(makeBlockResponse({ id: 'v-1' }))

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)

    // Type a query with both text and a filter. The free-text part
    // ("foo") matches both blocks; the `status:todo` filter keeps
    // only b1 (whose structured `properties` carries
    // `{ key: 'status', value: 'todo' }`). The DSL stored on the new
    // query block must be the full "foo status:: todo", not just
    // the text part.
    await userEvent.type(input, 'foo status:todo')

    await waitFor(() => {
      // b1 visible (status:todo) — content is just "this is a foo in the wild"
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
      // b2 hidden (status:done)
      expect(screen.queryByText('another foo')).not.toBeInTheDocument()
    })

    await userEvent.click(screen.getAllByTestId(/^save-as-view-/)[0])
    fireEvent.change(screen.getByTestId('save-view-name-input'), {
      target: { value: 'Tasks' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Bar' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    await waitFor(() => {
      expect(mockCreateBlock).toHaveBeenCalledTimes(2)
    })

    const [queryCallArgs] = mockCreateBlock.mock.calls[0]
    expect(queryCallArgs.properties.dsl).toBe('foo status:: todo')
  })

  it('closes the save modal after a successful submit', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)
    mockCreateBlock
      .mockResolvedValueOnce(makeBlockResponse({ id: 'q-1' }))
      .mockResolvedValueOnce(makeBlockResponse({ id: 'v-1' }))

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    await userEvent.click(screen.getAllByTestId(/^save-as-view-/)[0])
    fireEvent.change(screen.getByTestId('save-view-name-input'), {
      target: { value: 'Foo' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Bar' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    // After the two createBlock calls settle, the modal must be gone.
    await waitFor(() => {
      expect(screen.queryByTestId('save-view-name-input')).not.toBeInTheDocument()
    })
  })

  it('keeps the search modal open and shows an error when the createBlock call fails', async () => {
    mockListPages.mockResolvedValue(PAGES)
    mockSearchBlocks.mockResolvedValue(BLOCKS)
    mockCreateBlock.mockRejectedValue(new Error('Server exploded'))

    renderModal()
    const input = screen.getByPlaceholderText(/Search pages and blocks/i)
    await userEvent.type(input, 'foo')

    await waitFor(() => {
      expect(screen.getByText('this is a foo in the wild')).toBeInTheDocument()
    })

    await userEvent.click(screen.getAllByTestId(/^save-as-view-/)[0])
    fireEvent.change(screen.getByTestId('save-view-name-input'), {
      target: { value: 'Foo' },
    })
    fireEvent.change(screen.getByTestId('save-view-page-select'), {
      target: { value: 'Bar' },
    })

    await userEvent.click(screen.getByTestId('save-view-submit'))

    // The save modal is still mounted and shows the error.
    await waitFor(() => {
      expect(screen.getByTestId('save-view-error')).toBeInTheDocument()
    })
    expect(screen.getByTestId('save-view-error')).toHaveTextContent('Server exploded')
  })
})
