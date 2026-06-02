import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { PageAutocomplete } from '../PageAutocomplete'
import { api } from '@core/api-client'

// ──── API client mock ──────────────────────────────────────────────
// The component fetches the page list once on mount. We stub it so
// the test doesn't need a backend.

vi.mock('@core/api-client', () => ({
  api: {
    listPages: vi.fn().mockResolvedValue([
      { id: '1', name: 'Foo', title: 'Foo Page', journal: false, journalDay: null, createdAt: '' },
      { id: '2', name: 'Bar', title: 'Bar Page', journal: false, journalDay: null, createdAt: '' },
      { id: '3', name: 'Baz', title: null,        journal: false, journalDay: null, createdAt: '' },
    ]),
  },
}))

beforeEach(() => {
  vi.mocked(api.listPages).mockClear()
})

describe('PageAutocomplete', () => {
  it('renders nothing when position is null', async () => {
    const { container } = render(
      <PageAutocomplete
        position={null}
        query="Ba"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // The useEffect that calls api.listPages() fires after mount and
    // resolves asynchronously; wait for it so React's act() boundary
    // stays satisfied even though the test renders nothing.
    await waitFor(() => {
      expect(api.listPages).toHaveBeenCalled()
    })
    expect(container.firstChild).toBeNull()
  })

  it('filters pages by query (case-insensitive, matches name and title)', async () => {
    render(
      <PageAutocomplete
        position={{ top: 0, left: 0 }}
        query="ba"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // "Foo Page" should be filtered out; "Bar Page" and "Baz" should
    // match (the renderer falls back to name when title is null).
    await waitFor(() => {
      expect(screen.getByText('Bar Page')).toBeInTheDocument()
    })
    expect(screen.queryByText('Foo Page')).not.toBeInTheDocument()
    expect(screen.getByText('Baz')).toBeInTheDocument()
  })

  it('renders nothing when the filter produces zero results', async () => {
    render(
      <PageAutocomplete
        position={{ top: 0, left: 0 }}
        query="zzz-nothing-matches"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // The component returns null when pages.length === 0.
    // waitFor keeps the assertion async-friendly even when the
    // post-filter set is empty.
    await waitFor(() => {
      expect(api.listPages).toHaveBeenCalled()
    })
    expect(screen.queryByText('Foo Page')).not.toBeInTheDocument()
  })

  it('calls onSelect with the page name when a result is clicked', async () => {
    const onSelect = vi.fn()
    render(
      <PageAutocomplete
        position={{ top: 0, left: 0 }}
        query="Foo"
        onSelect={onSelect}
        onClose={vi.fn()}
      />,
    )
    await waitFor(() => screen.getByText('Foo Page'))
    await userEvent.click(screen.getByText('Foo Page'))
    expect(onSelect).toHaveBeenCalledWith('Foo')
  })

  it('calls onClose when the user mouses down outside the dropdown', async () => {
    const onClose = vi.fn()
    render(
      <div>
        <button data-testid="outside">outside</button>
        <PageAutocomplete
          position={{ top: 0, left: 0 }}
          query="Foo"
          onSelect={vi.fn()}
          onClose={onClose}
        />
      </div>,
    )
    await waitFor(() => screen.getByText('Foo Page'))
    // The component listens for `mousedown` on document, not click.
    fireEvent.mouseDown(screen.getByTestId('outside'))
    expect(onClose).toHaveBeenCalled()
  })
})
