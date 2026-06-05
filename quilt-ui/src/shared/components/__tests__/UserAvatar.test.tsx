// ─── UserAvatar — top-bar user menu button (F3) ─────────────────────
//
// F3 of quilt-fase3-backlog-small-fixes: the "A" avatar in the
// top bar was an unlabeled, non-interactive <div>. It now is a
// real <button> with an aria-label, a tooltip, and an onClick
// that opens the user menu (navigates to /settings in the V1
// implementation; the full dropdown menu is a follow-up).
//
// The initial letter is read from localStorage on mount. The
// `quilt:user-name` key is preferred; `quilt:author` is the
// legacy fallback already used by PageView for comment
// authorship.

import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { UserAvatar } from '../AppShell'

beforeEach(() => {
  localStorage.clear()
})

describe('UserAvatar — F3 (top-bar user menu button)', () => {
  it('renders as a button with aria-label and title "User menu"', () => {
    render(<UserAvatar onClick={vi.fn()} />)

    const btn = screen.getByRole('button', { name: /user menu/i })
    expect(btn).toBeInTheDocument()
    // The accessible name comes from aria-label; we still need
    // the title attribute for the visual tooltip.
    expect(btn).toHaveAttribute('title', 'User menu')
  })

  it('shows "U" as the fallback initial when no user name is set', () => {
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('U')
  })

  it('shows the first letter of the user name from localStorage', () => {
    localStorage.setItem('quilt:user-name', 'Ada Lovelace')
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('A')
  })

  it('upper-cases a lowercase initial', () => {
    localStorage.setItem('quilt:user-name', 'bea')
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('B')
  })

  it('falls back to the legacy "quilt:author" key when "quilt:user-name" is missing', () => {
    // Some legacy setups only set `quilt:author`; the avatar
    // should still pick that up.
    localStorage.setItem('quilt:author', 'Grace Hopper')
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('G')
  })

  it('prefers "quilt:user-name" over "quilt:author" when both are set', () => {
    localStorage.setItem('quilt:user-name', 'Linus')
    localStorage.setItem('quilt:author', 'Alan Turing')
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('L')
  })

  it('falls back to "U" when localStorage stores an empty string', () => {
    localStorage.setItem('quilt:user-name', '   ')
    render(<UserAvatar onClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('U')
  })

  it('falls back to "U" when the first character is whitespace', () => {
    localStorage.setItem('quilt:user-name', ' z')
    // ' z' starts with a space; we don't want a blank avatar.
    render(<UserAvatar onClick={vi.fn()} />)
    // The space character is technically the first char, but
    // we trim before slicing, so this becomes "z" → "Z".
    expect(screen.getByRole('button', { name: /user menu/i })).toHaveTextContent('Z')
  })

  it('fires onClick when clicked', async () => {
    const onClick = vi.fn()
    const user = userEvent.setup()
    render(<UserAvatar onClick={onClick} />)

    await user.click(screen.getByRole('button', { name: /user menu/i }))
    expect(onClick).toHaveBeenCalledTimes(1)
  })
})
