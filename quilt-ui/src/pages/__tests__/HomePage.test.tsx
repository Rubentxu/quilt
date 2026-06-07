// HomePage — root `/` should redirect to today's journal (YYYY-MM-DD).
//
// F1 of P0 frontend fixes: visiting `/` used to render `null`, leaving
// the user staring at an empty shell. The route is supposed to land
// them on the journal for "today" — the same date format the
// `/journal/$date` route accepts (e.g. `2026-06-05`).
//
// We assert the BEHAVIOR (navigation was triggered with the right
// target) — not the implementation detail of which React hook the
// component uses to schedule the navigation.

import { render } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { HomePage } from '../HomePage'

// ── Mocks ───────────────────────────────────────────────────────────

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ── Helpers ─────────────────────────────────────────────────────────

/** Local-tz YYYY-MM-DD for "today", matching what HomePage should produce. */
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

afterEach(() => {
  vi.useRealTimers()
})

// ── Tests ───────────────────────────────────────────────────────────

describe('HomePage — root route redirects to today\'s journal', () => {
  it('renders nothing visible (null) on `/`', () => {
    const { container } = render(<HomePage />)
    // The redirect component shouldn't paint anything to the DOM —
    // its job is to fire the navigation, then unmount.
    expect(container.firstChild).toBeNull()
  })

  it('navigates to /journal/$date with today\'s date in YYYY-MM-DD on mount', () => {
    render(<HomePage />)

    expect(mockNavigate).toHaveBeenCalledTimes(1)
    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/journal/$date',
      params: { date: expectedTodayIso() },
    })
  })

  it('uses the actual current date (not a hardcoded value)', () => {
    // Pin the clock to a known instant so we can prove the redirect
    // reads `new Date()` and not a stale constant.
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-05T10:30:00Z'))

    render(<HomePage />)

    expect(mockNavigate).toHaveBeenCalledWith({
      to: '/journal/$date',
      params: { date: '2026-06-05' },
    })
  })
})
