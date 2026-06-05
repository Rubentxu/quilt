// ─── JournalDateHeader — "Hoy" button navigates to today (F1) ───────
//
// F1 of quilt-fase3-backlog-small-fixes: the "Hoy" button in the
// journal date header was rendering but had no onClick. It should
// navigate to `/journal/<today>` (YYYY-MM-DD) when clicked.
//
// We export `JournalDateHeader` from PageView.tsx so the test can
// mount it in isolation — the parent PageView is too large (and
// pulls in WASM, SSE, dnd-kit, history) to be a useful test
// harness for a single button.

import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { JournalDateHeader } from '../PageView'

// ── Mocks ───────────────────────────────────────────────────────────

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ── Helpers ─────────────────────────────────────────────────────────

/**
 * Today's date in the route format (YYYY-MM-DD), in the local
 * timezone. This is the same format the `/journal/$date` route
 * accepts — we just compare by the ISO-style date string the
 * component produces, NOT by `new Date()` identity, because the
 * component reads `new Date()` at click time, not at render time.
 */
function expectedTodayIso(): string {
  const d = new Date()
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

beforeEach(() => {
  mockNavigate.mockReset()
})

// ── Tests ───────────────────────────────────────────────────────────

describe('JournalDateHeader — F1 (Hoy button navigates to today)', () => {
  it('renders the "Hoy" button', () => {
    render(<JournalDateHeader pageName="2026-06-05" />)
    expect(screen.getByRole('button', { name: /today/i })).toBeInTheDocument()
  })

  it('clicking "Hoy" navigates to /journal/$date with today in YYYY-MM-DD', async () => {
    const user = userEvent.setup()
    render(<JournalDateHeader pageName="2026-06-05" />)

    const hoy = screen.getByRole('button', { name: /today/i })
    await user.click(hoy)

    expect(mockNavigate).toHaveBeenCalledTimes(1)
    // The exact call shape (to + params) — the route is the
    // journal date route and the date is today in YYYY-MM-DD.
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/journal/$date',
      params: { date: expectedTodayIso() },
    })
  })

  it('clicking "Hoy" from a non-today journal still navigates to TODAY (not the visible date)', async () => {
    // The visible date is "yesterday" — but the user is on the
    // "Prev" / "Next" day navigation, and "Hoy" should always
    // jump to today, not the page they happen to be viewing.
    const user = userEvent.setup()
    render(<JournalDateHeader pageName="2020-01-01" />)

    const hoy = screen.getByRole('button', { name: /today/i })
    await user.click(hoy)

    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/journal/$date',
      params: { date: expectedTodayIso() },
    })
  })
})
