import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { CardRenderer, type BlockCard } from '../CardRenderer'

// Lightweight smoke tests for the CardRenderer. The detailed behavior
// (open button, copy link, collapse) is covered indirectly via
// existing PageView tests; here we just verify the shape dispatcher
// and the data attributes that the user CSS hooks into.

describe('CardRenderer (ADR-0007)', () => {
  const baseProps = {
    title: 'My title',
    children: <span>inner content</span>,
  }

  it('renders children only when card is null', () => {
    render(
      <CardRenderer {...baseProps} card={null as unknown as BlockCard} />,
    )
    expect(screen.getByText('inner content')).toBeInTheDocument()
    expect(screen.queryByTestId('card-renderer')).toBeNull()
  })

  it('renders the reference shape with data-shape and data-template attrs', () => {
    const card: BlockCard = {
      shape: 'reference',
      icon: '🔗',
      templateName: 'reference',
    }
    render(<CardRenderer {...baseProps} card={card} metas={[{ key: 'author', value: 'claude' }]} />)
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('data-shape')).toBe('reference')
    expect(root.getAttribute('data-template')).toBe('reference')
    expect(screen.getByText('author:')).toBeInTheDocument()
    expect(screen.getByText('claude')).toBeInTheDocument()
  })

  it('renders the content shape with collapse toggle', () => {
    const card: BlockCard = {
      shape: 'content',
      icon: '📄',
      templateName: 'documentation',
    }
    render(<CardRenderer {...baseProps} card={card} />)
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('data-shape')).toBe('content')
    // The "Expand section" / "Collapse section" button is rendered
    expect(screen.getByRole('button', { name: /collapse section/i })).toBeInTheDocument()
  })

  it('applies cssclass to the wrapper for user-defined styling', () => {
    const card: BlockCard = {
      shape: 'inline',
      icon: '🎯',
      cssclass: 'my-custom-class',
      templateName: 'meeting-notes',
    }
    const { container } = render(<CardRenderer {...baseProps} card={card} />)
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('class')).toBe('my-custom-class')
    expect(root.getAttribute('data-template')).toBe('meeting-notes')
    // Sanity check: container is not empty
    expect(container.firstChild).not.toBeNull()
  })

  it('falls back to inline shape with a warning when the shape is unknown', () => {
    const card = {
      shape: 'bogus',
      templateName: 'broken',
    } as unknown as BlockCard
    // Suppress the expected console.warn
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    render(<CardRenderer {...baseProps} card={card} />)
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining('Unknown card-shape "bogus"'),
    )
    warn.mockRestore()
  })

  // T-22: F14 extends CardShape with kanban-card and timeline-card.
  // V1: both new shapes render via InlineShape (placeholder) but
  // preserve the data-shape attribute so user CSS can hook into them.
  it('renders kanban-card shape (V1 placeholder) preserving data-shape', () => {
    const card: BlockCard = {
      shape: 'kanban-card',
      templateName: 'kanban-task',
    }
    const { container } = render(<CardRenderer {...baseProps} card={card} />)
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('data-shape')).toBe('kanban-card')
    expect(root.getAttribute('data-template')).toBe('kanban-task')
    expect(container.firstChild).not.toBeNull()
  })

  it('renders timeline-card shape (V1 placeholder) preserving data-shape', () => {
    const card: BlockCard = {
      shape: 'timeline-card',
      templateName: 'timeline-event',
    }
    const { container } = render(<CardRenderer {...baseProps} card={card} />)
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('data-shape')).toBe('timeline-card')
    expect(root.getAttribute('data-template')).toBe('timeline-event')
    expect(container.firstChild).not.toBeNull()
  })

  // T-22: pre-change shapes are byte-identical — reference shape still
  // renders with all meta and template attrs.
  it('reference shape is unchanged after CardShape extension (regression)', () => {
    const card: BlockCard = {
      shape: 'reference',
      icon: '🔗',
      templateName: 'reference',
    }
    render(
      <CardRenderer {...baseProps} card={card} metas={[{ key: 'author', value: 'alice' }]} />,
    )
    const root = screen.getByTestId('card-renderer')
    expect(root.getAttribute('data-shape')).toBe('reference')
    expect(screen.getByText('author:')).toBeInTheDocument()
    expect(screen.getByText('alice')).toBeInTheDocument()
  })
})
