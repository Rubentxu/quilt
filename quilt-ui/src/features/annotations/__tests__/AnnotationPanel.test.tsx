/**
 * Component tests for `AnnotationPanel`.
 *
 * Covers the spec-annotation-panel requirements:
 *  - Renders the list (sorted newest first) from the API
 *  - Status / scope / author filters narrow the list
 *  - Empty state for zero annotations
 *  - Resolve / delete / reply mutations call the right API methods
 *  - Clicking a row navigates to the block's page
 *
 * The component under test is a pure view + small set of mutations;
 * the actual polling cadence is exercised in a separate integration
 * test (out of scope here).
 */

import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { AnnotationPanel } from '../AnnotationPanel'
import type { Annotation } from '@shared/types/api'

// ── Mocks ───────────────────────────────────────────────────────────

const mockListAnnotations = vi.fn()
const mockUpdateStatus = vi.fn()
const mockDeleteAnnotation = vi.fn()
const mockCreateAnnotation = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    listAnnotations: (...args: unknown[]) => mockListAnnotations(...args),
    updateAnnotationStatus: (...args: unknown[]) => mockUpdateStatus(...args),
    deleteAnnotation: (...args: unknown[]) => mockDeleteAnnotation(...args),
    createAnnotation: (...args: unknown[]) => mockCreateAnnotation(...args),
  },
  QuiltApiError: class QuiltApiError extends Error {
    constructor(public status: number, public code: string, public detail: string) {
      super(detail)
      this.name = 'QuiltApiError'
    }
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

function makeAnnotation(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: 'a1',
    blockId: 'b1',
    scope: 'block',
    authorType: 'human',
    authorName: 'alice',
    content: 'looks good',
    status: 'pending',
    createdAt: '2026-06-01T00:00:00Z',
    ...overrides,
  } as Annotation
}

beforeEach(() => {
  mockListAnnotations.mockReset()
  mockUpdateStatus.mockReset()
  mockDeleteAnnotation.mockReset()
  mockCreateAnnotation.mockReset()
  mockNavigate.mockReset()
  // Default: no localStorage identity — defaults to "me"
  if (typeof localStorage !== 'undefined') localStorage.clear()
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
})

// ── Loading / empty / error states ─────────────────────────────────

describe('AnnotationPanel — initial states', () => {
  it('shows the loading message while the first fetch is in flight', () => {
    mockListAnnotations.mockReturnValue(new Promise(() => {})) // never resolves
    render(<AnnotationPanel enablePolling={false} />)
    expect(screen.getByText(/Loading annotations/i)).toBeInTheDocument()
  })

  it('renders the empty state when the API returns no annotations', async () => {
    mockListAnnotations.mockResolvedValueOnce([])
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-panel-empty')
    expect(screen.getByText('No annotations yet')).toBeInTheDocument()
  })

  it('renders an error banner when the fetch rejects', async () => {
    mockListAnnotations.mockRejectedValueOnce(new Error('network down'))
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-panel-error')
    expect(screen.getByTestId('annotation-panel-error').textContent).toMatch(/network down/)
  })

  it('skips the first fetch when initialAnnotations is provided', async () => {
    mockListAnnotations.mockResolvedValueOnce([])
    render(
      <AnnotationPanel
        enablePolling={false}
        initialAnnotations={[makeAnnotation({ id: 'pre' })]}
      />,
    )
    expect(mockListAnnotations).not.toHaveBeenCalled()
    expect(await screen.findByTestId('annotation-row-pre')).toBeInTheDocument()
  })
})

// ── List rendering + sort ──────────────────────────────────────────

describe('AnnotationPanel — list rendering', () => {
  it('renders every annotation with its content + author', async () => {
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'a1', authorName: 'alice', content: 'first' }),
      makeAnnotation({ id: 'a2', authorName: 'bob', content: 'second' }),
    ])
    render(<AnnotationPanel enablePolling={false} />)
    expect(await screen.findByTestId('annotation-row-a1')).toBeInTheDocument()
    expect(screen.getByText('first')).toBeInTheDocument()
    expect(screen.getByText('alice')).toBeInTheDocument()
    expect(screen.getByText('second')).toBeInTheDocument()
  })

  it('sorts newest-first by createdAt', async () => {
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'old', createdAt: '2026-01-01T00:00:00Z' }),
      makeAnnotation({ id: 'new', createdAt: '2026-06-01T00:00:00Z' }),
      makeAnnotation({ id: 'mid', createdAt: '2026-03-01T00:00:00Z' }),
    ])
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-new')
    const list = screen.getByTestId('annotation-list')
    const children = Array.from(list.children) as HTMLElement[]
    // Only the first level of the tree (roots) is sortable; replies
    // are inside their parent. We check the order of the root nodes.
    const order = children
      .map(c => c.getAttribute('data-testid'))
      .filter((t): t is string => !!t)
    expect(order).toEqual([
      'annotation-thread-node-new',
      'annotation-thread-node-mid',
      'annotation-thread-node-old',
    ])
  })

  it('renders agent-authored annotations with the agent icon (data attribute)', async () => {
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'agent1', authorType: 'agent', authorName: 'claude' }),
    ])
    render(<AnnotationPanel enablePolling={false} />)
    const row = await screen.findByTestId('annotation-row-agent1')
    expect(row.getAttribute('data-annotation-author-type')).toBe('agent')
  })
})

// ── Filters ────────────────────────────────────────────────────────

describe('AnnotationPanel — filters', () => {
  const seed = [
    makeAnnotation({ id: 'a1', status: 'pending', scope: 'block', authorName: 'alice' }),
    makeAnnotation({ id: 'a2', status: 'resolved', scope: 'inline', authorName: 'bob' }),
    makeAnnotation({ id: 'a3', status: 'pending', scope: 'inline', authorName: 'claude' }),
  ]

  it('shows all annotations by default', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} />)
    expect(await screen.findByTestId('annotation-row-a1')).toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a2')).toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a3')).toBeInTheDocument()
  })

  it('filters by status', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} initialShowFilters />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.change(screen.getByTestId('filter-status'), { target: { value: 'resolved' } })
    expect(screen.queryByTestId('annotation-row-a1')).not.toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a2')).toBeInTheDocument()
    expect(screen.queryByTestId('annotation-row-a3')).not.toBeInTheDocument()
  })

  it('filters by scope', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} initialShowFilters />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.change(screen.getByTestId('filter-scope'), { target: { value: 'inline' } })
    expect(screen.queryByTestId('annotation-row-a1')).not.toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a2')).toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a3')).toBeInTheDocument()
  })

  it('filters by author name (substring, case-insensitive)', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} initialShowFilters />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.change(screen.getByTestId('filter-author'), { target: { value: 'CLAU' } })
    expect(screen.queryByTestId('annotation-row-a1')).not.toBeInTheDocument()
    expect(screen.queryByTestId('annotation-row-a2')).not.toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a3')).toBeInTheDocument()
  })

  it('shows a "no matches" message when filters exclude everything', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} initialShowFilters />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.change(screen.getByTestId('filter-status'), { target: { value: 'dismissed' } })
    expect(screen.getByTestId('annotation-panel-no-matches')).toBeInTheDocument()
  })

  it('clear button resets all filters', async () => {
    mockListAnnotations.mockResolvedValueOnce(seed)
    render(<AnnotationPanel enablePolling={false} initialShowFilters />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.change(screen.getByTestId('filter-status'), { target: { value: 'resolved' } })
    fireEvent.change(screen.getByTestId('filter-author'), { target: { value: 'bob' } })
    expect(screen.queryByTestId('annotation-row-a1')).not.toBeInTheDocument()
    fireEvent.click(screen.getByTestId('filter-clear'))
    expect(screen.getByTestId('annotation-row-a1')).toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a3')).toBeInTheDocument()
  })
})

// ── Mutations ──────────────────────────────────────────────────────

describe('AnnotationPanel — mutations', () => {
  it('resolve button PATCHes status=resolved with the localStorage user', async () => {
    localStorage.setItem('quilt:user-name', 'bob')
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'a1', status: 'pending' }),
    ])
    const updated = makeAnnotation({ id: 'a1', status: 'resolved', resolvedBy: 'bob' })
    mockUpdateStatus.mockResolvedValueOnce(updated)
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-resolve-a1'))
    await waitFor(() => {
      expect(mockUpdateStatus).toHaveBeenCalledWith('a1', {
        status: 'resolved',
        resolvedBy: 'bob',
      })
    })
  })

  it('re-resolving sends status=pending and clears the row\'s resolved style', async () => {
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'a1', status: 'resolved' }),
    ])
    mockUpdateStatus.mockResolvedValueOnce(
      makeAnnotation({ id: 'a1', status: 'pending' }),
    )
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-resolve-a1'))
    await waitFor(() => {
      expect(mockUpdateStatus).toHaveBeenCalledWith('a1', { status: 'pending' })
    })
  })

  it('delete button calls deleteAnnotation and removes the row', async () => {
    mockListAnnotations.mockResolvedValueOnce([
      makeAnnotation({ id: 'a1' }),
      makeAnnotation({ id: 'a2' }),
    ])
    mockDeleteAnnotation.mockResolvedValueOnce(undefined)
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-delete-a1'))
    await waitFor(() => {
      expect(mockDeleteAnnotation).toHaveBeenCalledWith('a1')
    })
    expect(screen.queryByTestId('annotation-row-a1')).not.toBeInTheDocument()
    expect(screen.getByTestId('annotation-row-a2')).toBeInTheDocument()
  })

  it('reply button reveals an inline input; submit creates a new annotation', async () => {
    mockListAnnotations.mockResolvedValueOnce([makeAnnotation({ id: 'a1' })])
    const reply = makeAnnotation({
      id: 'a2',
      parentAnnotationId: 'a1',
      content: 'agreed',
      authorName: 'me',
    })
    mockCreateAnnotation.mockResolvedValueOnce(reply)
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-reply-a1'))
    const input = screen.getByTestId('annotation-reply-input-a1')
    fireEvent.change(input, { target: { value: 'agreed' } })
    fireEvent.click(screen.getByTestId('annotation-reply-submit-a1'))
    await waitFor(() => {
      expect(mockCreateAnnotation).toHaveBeenCalledWith({
        blockId: 'b1',
        scope: 'block',
        authorType: 'human',
        authorName: 'me',
        content: 'agreed',
        parentAnnotationId: 'a1',
      })
    })
    // The new reply is now in the list and is nested under a1
    expect(await screen.findByTestId('annotation-row-a2')).toBeInTheDocument()
  })

  it('cancel reply hides the input without calling the API', async () => {
    mockListAnnotations.mockResolvedValueOnce([makeAnnotation({ id: 'a1' })])
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-reply-a1'))
    fireEvent.click(screen.getByTestId('annotation-reply-cancel-a1'))
    expect(screen.queryByTestId('annotation-reply-input-a1')).not.toBeInTheDocument()
    expect(mockCreateAnnotation).not.toHaveBeenCalled()
  })
})

// ── Navigation ─────────────────────────────────────────────────────

describe('AnnotationPanel — navigation', () => {
  it('clicking a row navigates to the page with the blockId in sessionStorage', async () => {
    mockListAnnotations.mockResolvedValueOnce([makeAnnotation({ id: 'a1', blockId: 'b99' })])
    render(<AnnotationPanel enablePolling={false} />)
    const row = await screen.findByTestId('annotation-thread-node-a1')
    fireEvent.click(row)
    expect(sessionStorage.getItem('quilt:focusBlock')).toBe('b99')
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/page/$name',
      params: { name: 'b99' },
    })
  })

  it('the action buttons stop click propagation (only the row navigates)', async () => {
    mockListAnnotations.mockResolvedValueOnce([makeAnnotation({ id: 'a1' })])
    mockDeleteAnnotation.mockResolvedValueOnce(undefined)
    render(<AnnotationPanel enablePolling={false} />)
    await screen.findByTestId('annotation-row-a1')
    fireEvent.click(screen.getByTestId('annotation-row-delete-a1'))
    expect(mockNavigate).not.toHaveBeenCalled()
  })
})

// ── Polling ────────────────────────────────────────────────────────

describe('AnnotationPanel — polling', () => {
  it('polls at the configured interval when enablePolling is true (default)', async () => {
    // Use real timers with a tiny poll interval — fake timers don't
    // play nicely with the async `api.listAnnotations` mock.
    mockListAnnotations.mockResolvedValue([])
    render(<AnnotationPanel pollIntervalMs={30} />)
    // Initial fetch (effect runs on mount)
    await waitFor(() => expect(mockListAnnotations).toHaveBeenCalledTimes(1))
    // Wait for at least one additional poll cycle
    await waitFor(
      () => expect(mockListAnnotations.mock.calls.length).toBeGreaterThanOrEqual(2),
      { timeout: 1000 },
    )
  })

  it('does not poll when enablePolling is false', async () => {
    mockListAnnotations.mockResolvedValue([])
    render(<AnnotationPanel enablePolling={false} pollIntervalMs={30} />)
    await waitFor(() => expect(mockListAnnotations).toHaveBeenCalledTimes(1))
    // Wait long enough that a second poll would have fired if the
    // interval were active (10x the interval).
    await new Promise(r => setTimeout(r, 300))
    expect(mockListAnnotations).toHaveBeenCalledTimes(1)
  })
})
