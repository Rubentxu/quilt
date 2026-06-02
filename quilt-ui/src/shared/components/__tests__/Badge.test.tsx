import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { Badge } from '../Badge'

describe('Badge', () => {
  it('renders children', () => {
    render(<Badge>TODO</Badge>)
    expect(screen.getByText('TODO')).toBeInTheDocument()
  })

  it('uses primary-container background by default', () => {
    const { container } = render(<Badge>Default</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.background).toBe('var(--color-primary-container)')
    expect(badge.style.color).toBe('var(--color-primary)')
  })

  it('applies success variant', () => {
    const { container } = render(<Badge variant="success">Done</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.background).toBe('var(--color-success-subtle)')
    expect(badge.style.color).toBe('var(--color-success)')
  })

  it('applies warning variant', () => {
    const { container } = render(<Badge variant="warning">Blocked</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.background).toBe('var(--color-warning-subtle)')
    expect(badge.style.color).toBe('var(--color-warning)')
  })

  it('applies danger variant', () => {
    const { container } = render(<Badge variant="danger">Error</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.background).toBe('var(--color-danger-subtle)')
    expect(badge.style.color).toBe('var(--color-danger)')
  })

  it('applies info variant', () => {
    const { container } = render(<Badge variant="info">Note</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.background).toBe('var(--color-info-subtle)')
    expect(badge.style.color).toBe('var(--color-info)')
  })

  it('uses pill radius (DESIGN.md §8.2)', () => {
    const { container } = render(<Badge>Pill</Badge>)
    const badge = container.firstChild as HTMLElement
    expect(badge.style.borderRadius).toBe('var(--radius-pill)')
  })

  it('renders title attribute when provided', () => {
    render(<Badge title="Tooltip">Tag</Badge>)
    expect(screen.getByText('Tag')).toHaveAttribute('title', 'Tooltip')
  })
})
