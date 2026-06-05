import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { GroupHeader } from '../sections/GroupHeader'

// Approval tests for GroupHeader (DESIGN.md §9.1).
// These pin the visual contract: the section header is an h3 with
// uppercase / muted styling, and it disappears entirely when the
// sidebar is collapsed. The original implementation lived inside
// Sidebar.tsx; this test was added BEFORE the extraction so we can
// catch any behavioral drift during the move.

describe('GroupHeader — DESIGN.md §9.1', () => {
  it('renders the label as an h3', () => {
    render(<GroupHeader label="Páginas" />)
    const heading = screen.getByRole('heading', { name: 'Páginas' })
    expect(heading).toBeInTheDocument()
    expect(heading.tagName).toBe('H3')
  })

  it('renders uppercase styling on the heading (DESIGN.md §9.1 — section header style)', () => {
    render(<GroupHeader label="Favoritos" />)
    const heading = screen.getByRole('heading', { name: 'Favoritos' })
    expect(heading.style.textTransform).toBe('uppercase')
    expect(heading.style.fontSize).toBe('11px')
    expect(heading.style.fontWeight).toBe('600')
    expect(heading.style.letterSpacing).toBe('0.05em')
    expect(heading.style.color).toBe('var(--color-text-muted)')
  })

  it('renders when collapsed is false (explicit)', () => {
    render(<GroupHeader label="Diarios" collapsed={false} />)
    expect(screen.getByRole('heading', { name: 'Diarios' })).toBeInTheDocument()
  })

  it('returns null when collapsed is true (sidebar collapsed)', () => {
    const { container } = render(<GroupHeader label="Páginas" collapsed={true} />)
    expect(container.firstChild).toBeNull()
    expect(screen.queryByRole('heading', { name: 'Páginas' })).not.toBeInTheDocument()
  })

  it('treats undefined collapsed as "not collapsed" and still renders', () => {
    render(<GroupHeader label="Recientes" />)
    expect(screen.getByRole('heading', { name: 'Recientes' })).toBeInTheDocument()
  })
})
