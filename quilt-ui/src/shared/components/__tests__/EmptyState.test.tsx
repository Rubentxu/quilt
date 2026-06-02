import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { Inbox } from 'lucide-react'
import { EmptyState } from '../EmptyState'

describe('EmptyState — DESIGN.md §15', () => {
  it('renders the title', () => {
    render(<EmptyState title="No pages yet" />)
    expect(screen.getByRole('heading', { name: 'No pages yet' })).toBeInTheDocument()
  })

  it('renders the description when provided', () => {
    render(
      <EmptyState
        title="Empty"
        description="This is a description of the empty state."
      />
    )
    expect(screen.getByText(/This is a description/)).toBeInTheDocument()
  })

  it('renders the action when provided', () => {
    render(
      <EmptyState
        title="Empty"
        action={<button>Create page</button>}
      />
    )
    expect(screen.getByRole('button', { name: 'Create page' })).toBeInTheDocument()
  })

  it('uses Inbox icon by default', () => {
    const { container } = render(<EmptyState title="Empty" />)
    // Inbox icon is rendered as SVG inside a circle
    const svg = container.querySelector('svg')
    expect(svg).toBeInTheDocument()
  })

  it('accepts a custom icon', () => {
    const { container } = render(
      <EmptyState title="Custom" icon={<Inbox data-testid="custom-icon" />} />
    )
    expect(screen.getByTestId('custom-icon')).toBeInTheDocument()
  })

  it('has role="status" for assistive technology', () => {
    render(<EmptyState title="Empty" />)
    expect(screen.getByRole('status')).toBeInTheDocument()
  })

  it('action button is clickable', async () => {
    const user = userEvent.setup()
    const onClick = vi.fn()
    render(
      <EmptyState
        title="Empty"
        action={<button onClick={onClick}>Click me</button>}
      />
    )
    await user.click(screen.getByRole('button', { name: 'Click me' }))
    expect(onClick).toHaveBeenCalledTimes(1)
  })
})
