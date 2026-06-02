import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi } from 'vitest'
import { FloatingHelpButton } from '../FloatingHelpButton'

describe('FloatingHelpButton — DESIGN.md §9.10', () => {
  it('renders a button with aria-label', () => {
    render(<FloatingHelpButton label="Help" />)
    expect(screen.getByRole('button', { name: 'Help' })).toBeInTheDocument()
  })

  it('uses default label "Help & shortcuts" when none provided', () => {
    render(<FloatingHelpButton />)
    expect(screen.getByRole('button', { name: /Help & shortcuts/ })).toBeInTheDocument()
  })

  it('calls onClick when no panel is provided', async () => {
    const user = userEvent.setup()
    const onClick = vi.fn()
    render(<FloatingHelpButton onClick={onClick} />)
    await user.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('toggles the panel when one is provided', async () => {
    const user = userEvent.setup()
    render(
      <FloatingHelpButton
        label="Help"
        panel={<div>Panel content</div>}
      />
    )
    // Panel not visible initially
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    // Click to expand
    await user.click(screen.getByRole('button'))
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByText('Panel content')).toBeInTheDocument()

    // aria-expanded is true
    expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true')

    // Click again to collapse
    await user.click(screen.getByRole('button'))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('is keyboard accessible', () => {
    render(<FloatingHelpButton onClick={vi.fn()} />)
    const button = screen.getByRole('button')
    button.focus()
    expect(button).toHaveFocus()
  })
})
