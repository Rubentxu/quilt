import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { TagAutocomplete } from '../TagAutocomplete'

describe('TagAutocomplete', () => {
  it('renders nothing when position is null', () => {
    const { container } = render(
      <TagAutocomplete
        position={null}
        query="to"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders the full default tag list when query is empty', () => {
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // All 8 default tags should be visible.
    expect(screen.getByText('todo')).toBeInTheDocument()
    expect(screen.getByText('bug')).toBeInTheDocument()
    expect(screen.getByText('urgent')).toBeInTheDocument()
    expect(screen.getByText('wip')).toBeInTheDocument()
    expect(screen.getByText('idea')).toBeInTheDocument()
    expect(screen.getByText('question')).toBeInTheDocument()
    expect(screen.getByText('important')).toBeInTheDocument()
    expect(screen.getByText('done')).toBeInTheDocument()
  })

  it('filters tags by case-insensitive prefix', () => {
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query="U"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // "U" matches "urgent" and "important" (case-insensitive startsWith).
    expect(screen.getByText('urgent')).toBeInTheDocument()
    expect(screen.queryByText('todo')).not.toBeInTheDocument()
    expect(screen.queryByText('bug')).not.toBeInTheDocument()
  })

  it('returns null when the filter produces zero results', () => {
    const { container } = render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query="zzz-no-match"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('uses role=listbox and role=option for accessibility', () => {
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    expect(screen.getByRole('listbox')).toBeInTheDocument()
    const options = screen.getAllByRole('option')
    // 8 default tags = 8 options.
    expect(options).toHaveLength(8)
    // First option is selected by default.
    expect(options[0]).toHaveAttribute('aria-selected', 'true')
  })

  it('calls onSelect with the tag name when a result is clicked', () => {
    const onSelect = vi.fn()
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={onSelect}
        onClose={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByTestId('tag-option-urgent'))
    expect(onSelect).toHaveBeenCalledWith('urgent')
  })

  // ──── Keyboard navigation ──────────────────────────────────────

  it('ArrowDown moves the selection to the next tag', () => {
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // Default selection is index 0 ("todo"). ArrowDown → index 1 ("bug").
    fireEvent.keyDown(document, { key: 'ArrowDown' })
    const options = screen.getAllByRole('option')
    expect(options[0]).toHaveAttribute('aria-selected', 'false')
    expect(options[1]).toHaveAttribute('aria-selected', 'true')
  })

  it('ArrowUp wraps from the first item to the last', () => {
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // Default selection is index 0; ArrowUp should wrap to the last.
    fireEvent.keyDown(document, { key: 'ArrowUp' })
    const options = screen.getAllByRole('option')
    expect(options[options.length - 1]).toHaveAttribute('aria-selected', 'true')
    expect(options[0]).toHaveAttribute('aria-selected', 'false')
  })

  it('Enter selects the currently highlighted tag', () => {
    const onSelect = vi.fn()
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={onSelect}
        onClose={vi.fn()}
      />,
    )
    // Default selection is index 0 → "todo".
    fireEvent.keyDown(document, { key: 'Enter' })
    expect(onSelect).toHaveBeenCalledWith('todo')
  })

  it('Escape calls onClose', () => {
    const onClose = vi.fn()
    render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={onClose}
      />,
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalled()
  })

  it('calls onClose when the user mouses down outside the dropdown', async () => {
    const onClose = vi.fn()
    render(
      <div>
        <button data-testid="outside">outside</button>
        <TagAutocomplete
          position={{ top: 0, left: 0 }}
          query=""
          onSelect={vi.fn()}
          onClose={onClose}
        />
      </div>,
    )
    // The component listens for `mousedown` on document, not click.
    fireEvent.mouseDown(screen.getByTestId('outside'))
    expect(onClose).toHaveBeenCalled()
  })

  it('resets selection to the first item when the result set changes', async () => {
    const { rerender } = render(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query=""
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    // Move selection down twice → index 2 ("wip").
    fireEvent.keyDown(document, { key: 'ArrowDown' })
    fireEvent.keyDown(document, { key: 'ArrowDown' })
    let options = screen.getAllByRole('option')
    expect(options[2]).toHaveAttribute('aria-selected', 'true')

    // Narrow the query so only one tag remains. Selection should reset.
    rerender(
      <TagAutocomplete
        position={{ top: 0, left: 0 }}
        query="que"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    )
    await waitFor(() => {
      options = screen.getAllByRole('option')
      expect(options[0]).toHaveAttribute('aria-selected', 'true')
    })
    expect(screen.getByText('question')).toBeInTheDocument()
    expect(options).toHaveLength(1)
  })
})
