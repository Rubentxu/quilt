// ─── WeeklyReview component tests ────────────────────────────────────────────
//
// CG-7: Weekly Review end-to-end.

import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { WeeklyReview } from '../WeeklyReview'
import { api } from '@core/api-client'

// ─── Mock API ────────────────────────────────────────────────────────────────

const mockDto = {
  weekStart: '2026-06-09T10:00:00Z',
  weekEnd: '2026-06-16T10:00:00Z',
  blocksCreated: 5,
  blocksUpdated: 12,
  tasksCompleted: 3,
  decayTrend: 'worsening' as const,
  decayDelta: -3,
  journalDays: 3,
  suggestions: [
    'Review 2 stale blocks (high decay)',
    'Add at least 3 journal entries next week',
  ],
  generatedAt: new Date().toISOString(),
}

const mockEmptyDto = {
  weekStart: '2026-06-09T10:00:00Z',
  weekEnd: '2026-06-16T10:00:00Z',
  blocksCreated: 0,
  blocksUpdated: 0,
  tasksCompleted: 0,
  decayTrend: 'stable' as const,
  decayDelta: 0,
  journalDays: 0,
  suggestions: [],
  generatedAt: new Date().toISOString(),
}

vi.mock('@core/api-client', () => ({
  api: {
    getWeeklyReview: vi.fn(),
  },
}))

describe('WeeklyReview', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── Loading state ─────────────────────────────────────────────────────

  it('shows loading state initially', async () => {
    vi.mocked(api.getWeeklyReview).mockImplementation(
      () => new Promise(() => {}), // never resolves
    )

    render(<WeeklyReview />)

    expect(screen.getByTestId('weekly-review-loading')).toBeInTheDocument()
    expect(screen.getByText('Loading review…')).toBeInTheDocument()
  })

  // ─── Error state ───────────────────────────────────────────────────────

  it('shows error state when API fails', async () => {
    vi.mocked(api.getWeeklyReview).mockRejectedValue(new Error('Network error'))

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-error')).toBeInTheDocument()
    })
    expect(screen.getByText('Network error')).toBeInTheDocument()
  })

  // ─── Empty state ───────────────────────────────────────────────────────

  it('shows empty state when all counters are 0', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockEmptyDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review')).toBeInTheDocument()
    })

    expect(screen.getByTestId('weekly-review-empty')).toBeInTheDocument()
    expect(
      screen.getByText('Start journaling to see your weekly review'),
    ).toBeInTheDocument()
  })

  // ─── With data — first step ────────────────────────────────────────────

  it('renders step 1 (Numbers) on first load', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })
    expect(screen.getByTestId('weekly-review-next')).toBeInTheDocument()
  })

  // ─── Counters ──────────────────────────────────────────────────────────

  it('renders counter values on step 1', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    expect(
      screen.getByTestId('weekly-review-counter-blocksCreated'),
    ).toHaveTextContent('5')
    expect(
      screen.getByTestId('weekly-review-counter-blocksUpdated'),
    ).toHaveTextContent('12')
    expect(
      screen.getByTestId('weekly-review-counter-tasksCompleted'),
    ).toHaveTextContent('3')
    expect(
      screen.getByTestId('weekly-review-counter-journalDays'),
    ).toHaveTextContent('3')
  })

  // ─── Trend rendering ───────────────────────────────────────────────────

  it('shows the worsening trend on step 2', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('weekly-review-next'))

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-2')).toBeInTheDocument()
    })

    expect(screen.getByTestId('weekly-review-trend')).toHaveTextContent(
      'worsening',
    )
    expect(screen.getByTestId('weekly-review-delta').textContent).toContain(
      '3 more decay',
    )
  })

  // ─── Suggestion bullets ────────────────────────────────────────────────

  it('renders suggestion bullets on step 3', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('weekly-review-next')) // → 2
    await user.click(screen.getByTestId('weekly-review-next')) // → 3

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-3')).toBeInTheDocument()
    })

    expect(
      screen.getByTestId('weekly-review-suggestion-0'),
    ).toHaveTextContent('Review 2 stale blocks (high decay)')
    expect(
      screen.getByTestId('weekly-review-suggestion-1'),
    ).toHaveTextContent('Add at least 3 journal entries next week')
  })

  // ─── Step navigation ───────────────────────────────────────────────────

  it('first step has no Back button', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    expect(screen.queryByTestId('weekly-review-back')).not.toBeInTheDocument()
  })

  it('last step has Done instead of Next', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('weekly-review-next')) // → 2
    await user.click(screen.getByTestId('weekly-review-next')) // → 3
    await user.click(screen.getByTestId('weekly-review-next')) // → 4

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-4')).toBeInTheDocument()
    })

    expect(screen.queryByTestId('weekly-review-next')).not.toBeInTheDocument()
    expect(screen.getByTestId('weekly-review-done')).toBeInTheDocument()
  })

  it('Back button retreats to previous step', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('weekly-review-next')) // → 2

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-2')).toBeInTheDocument()
    })

    await user.click(screen.getByTestId('weekly-review-back')) // → 1

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })
  })

  it('Done returns to step 1', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('weekly-review-next'))
    await user.click(screen.getByTestId('weekly-review-next'))
    await user.click(screen.getByTestId('weekly-review-next'))

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-done')).toBeInTheDocument()
    })

    await user.click(screen.getByTestId('weekly-review-done'))

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })
  })

  // ─── Keyboard navigation ───────────────────────────────────────────────

  it('Enter advances to next step', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const root = screen.getByTestId('weekly-review') as HTMLElement
    root.focus()
    const user = userEvent.setup()
    await user.keyboard('{Enter}')

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-2')).toBeInTheDocument()
    })
  })

  it('ArrowLeft retreats to previous step', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-1')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    // Advance to step 3 via two Next clicks
    await user.click(screen.getByTestId('weekly-review-next'))
    await user.click(screen.getByTestId('weekly-review-next'))

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-3')).toBeInTheDocument()
    })

    const root = screen.getByTestId('weekly-review') as HTMLElement
    root.focus()
    await user.keyboard('{ArrowLeft}')

    await waitFor(() => {
      expect(screen.getByTestId('weekly-review-step-2')).toBeInTheDocument()
    })
  })

  // ─── Accessibility ─────────────────────────────────────────────────────

  it('has the Weekly Review region landmark', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Weekly Review' })).toBeInTheDocument()
    })
  })

  it('current step has a region with step name', async () => {
    vi.mocked(api.getWeeklyReview).mockResolvedValue(mockDto)

    render(<WeeklyReview />)

    await waitFor(() => {
      expect(
        screen.getByRole('region', { name: /Step 1 of 4: Numbers/ }),
      ).toBeInTheDocument()
    })
  })
})
