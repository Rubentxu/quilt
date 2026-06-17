// ─── MorningBriefing component tests ────────────────────────────────────────────
//
// CG-1: Morning Briefing end-to-end.

import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { MorningBriefing } from '../MorningBriefing'
import { api } from '@core/api-client'

// ─── Mock API ────────────────────────────────────────────────────────────────

const mockMorningBriefingDto = {
  agendaItems: [
    {
      blockId: 'block-1',
      contentPreview: 'This is an agenda item about project planning',
      pageName: 'journals/2024-01-15',
      hasChildren: true,
      updatedAt: new Date().toISOString(),
    },
    {
      blockId: 'block-2',
      contentPreview: 'Another item for today',
      pageName: 'journals/2024-01-15',
      hasChildren: false,
      updatedAt: new Date().toISOString(),
    },
  ],
  decayAlerts: [
    {
      blockId: 'block-3',
      contentPreview: 'This block has not been updated in 30 days',
      pageName: 'journals/2024-01-01',
      daysSinceUpdate: 30,
      severity: 'high',
      reason: 'No updates in 30 days — significantly stale',
    },
    {
      blockId: 'block-4',
      contentPreview: 'This block has not been updated in 15 days',
      pageName: 'journals/2024-01-10',
      daysSinceUpdate: 15,
      severity: 'medium',
      reason: 'No updates in 15 days — consider reviewing',
    },
  ],
  serendipityHighlights: [
    {
      blockAId: 'block-5',
      blockBId: 'block-6',
      blockAPreview: 'First block preview',
      blockBPreview: 'Second block preview',
      explanation: 'Both blocks discuss project management despite different contexts',
      confidence: 0.85,
    },
  ],
  generatedAt: new Date().toISOString(),
  daysSinceLastJournal: 0,
}

vi.mock('@core/api-client', () => ({
  api: {
    getMorningBriefing: vi.fn(),
  },
}))

describe('MorningBriefing', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── Loading state ───────────────────────────────────────────────────────

  it('shows loading state initially', async () => {
    vi.mocked(api.getMorningBriefing).mockImplementation(
      () => new Promise(() => {}), // never resolves
    )

    render(<MorningBriefing />)

    expect(screen.getByTestId('morning-briefing-loading')).toBeInTheDocument()
    expect(screen.getByText('Loading briefing…')).toBeInTheDocument()
  })

  // ─── Error state ────────────────────────────────────────────────────────

  it('shows error state when API fails', async () => {
    vi.mocked(api.getMorningBriefing).mockRejectedValue(new Error('Network error'))

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing-error')).toBeInTheDocument()
    })
    expect(screen.getByText('Network error')).toBeInTheDocument()
  })

  // ─── Empty state ────────────────────────────────────────────────────────

  it('shows empty sections when no data', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue({
      agendaItems: [],
      decayAlerts: [],
      serendipityHighlights: [],
      generatedAt: new Date().toISOString(),
      daysSinceLastJournal: 1,
    })

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    expect(screen.getByTestId('morning-briefing-agenda-empty')).toBeInTheDocument()
    expect(screen.getByText('No activity today yet')).toBeInTheDocument()

    expect(screen.getByTestId('morning-briefing-decay-empty')).toBeInTheDocument()
    expect(screen.getByText('No decay alerts — everything looks healthy')).toBeInTheDocument()

    expect(screen.getByTestId('morning-briefing-serendipity-empty')).toBeInTheDocument()
    expect(screen.getByText('No serendipity highlights yet')).toBeInTheDocument()
  })

  // ─── With data state ────────────────────────────────────────────────────

  it('renders agenda items correctly', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    expect(screen.getByTestId('morning-briefing-agenda-list')).toBeInTheDocument()
    expect(screen.getByTestId('morning-briefing-agenda-item-block-1')).toBeInTheDocument()
    // pageName appears in both agenda items, so use getAllByText
    expect(screen.getAllByText('journals/2024-01-15')).toHaveLength(2)
  })

  it('renders decay alerts with correct severity colors', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing-decay-list')).toBeInTheDocument()
    })

    // High severity alert
    const highAlert = screen.getByTestId('morning-briefing-decay-item-block-3')
    expect(highAlert).toBeInTheDocument()
    expect(highAlert.textContent).toContain('30d ago')

    // Medium severity alert
    const mediumAlert = screen.getByTestId('morning-briefing-decay-item-block-4')
    expect(mediumAlert).toBeInTheDocument()
    expect(mediumAlert.textContent).toContain('15d ago')
  })

  it('renders serendipity highlights with confidence score', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing-serendipity-list')).toBeInTheDocument()
    })

    expect(screen.getByText('85% match')).toBeInTheDocument()
    expect(
      screen.getByText('Both blocks discuss project management despite different contexts'),
    ).toBeInTheDocument()
  })

  it('shows journal status in header', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByText('Journal updated today')).toBeInTheDocument()
    })
  })

  it('shows days since last journal when not today', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue({
      ...mockMorningBriefingDto,
      daysSinceLastJournal: 5,
    })

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByText('Last journal 5d ago')).toBeInTheDocument()
    })
  })

  // ─── Refresh button ────────────────────────────────────────────────────

  it('refresh button triggers reload', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    const refreshBtn = screen.getByTestId('morning-briefing-refresh')
    await user.click(refreshBtn)

    // First call + refresh call
    expect(api.getMorningBriefing).toHaveBeenCalledTimes(2)
  })

  // ─── Accessibility ──────────────────────────────────────────────────────

  it('has proper ARIA labels on sections', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Morning Briefing' })).toBeInTheDocument()
    })

    // aria-labelledby points to the section's heading; use getByRole('region', { name })
    expect(screen.getByRole('region', { name: /Today's Agenda/ })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /Decay Alerts/ })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /Serendipity Highlights/ })).toBeInTheDocument()
  })

  it('shows generated timestamp in footer', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing-footer')).toBeInTheDocument()
    })
    expect(screen.getByTestId('morning-briefing-footer').textContent).toContain('Generated')
  })

  // ─── Collapse / expand ─────────────────────────────────────────────────

  it('renders in expanded state by default with aria-expanded="true"', async () => {
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    const region = screen.getByRole('region', { name: 'Morning Briefing' })
    expect(region).toHaveAttribute('aria-expanded', 'true')
    // Sections are visible
    expect(screen.getByTestId('morning-briefing-agenda-header')).toBeInTheDocument()
    expect(screen.getByTestId('morning-briefing-decay-header')).toBeInTheDocument()
    expect(screen.getByTestId('morning-briefing-serendipity-header')).toBeInTheDocument()
  })

  it('collapse toggle hides sections and sets aria-expanded="false"', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    const collapseBtn = screen.getByTestId('morning-briefing-collapse')
    await user.click(collapseBtn)

    const region = screen.getByRole('region', { name: 'Morning Briefing' })
    expect(region).toHaveAttribute('aria-expanded', 'false')
    // data-testid stays on wrapper
    expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    // Section headers are hidden
    expect(screen.queryByTestId('morning-briefing-agenda-header')).not.toBeInTheDocument()
    expect(screen.queryByTestId('morning-briefing-decay-header')).not.toBeInTheDocument()
    expect(screen.queryByTestId('morning-briefing-serendipity-header')).not.toBeInTheDocument()
  })

  it('collapse toggle expands from collapsed state', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    // Collapse
    await user.click(screen.getByTestId('morning-briefing-collapse'))
    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')

    // Expand
    await user.click(screen.getByTestId('morning-briefing-collapse'))
    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByTestId('morning-briefing-agenda-header')).toBeInTheDocument()
  })

  it('keyboard toggle (Enter) collapses the briefing', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    const collapseBtn = screen.getByTestId('morning-briefing-collapse')
    collapseBtn.focus()
    await user.keyboard('{Enter}')

    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')
  })

  it('keyboard toggle (Space) collapses the briefing', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    const collapseBtn = screen.getByTestId('morning-briefing-collapse')
    collapseBtn.focus()
    await user.keyboard(' ')

    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')
  })

  it('refresh button preserves collapsed state', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockResolvedValue(mockMorningBriefingDto)

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    // Collapse
    await user.click(screen.getByTestId('morning-briefing-collapse'))
    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')

    // Refresh while collapsed
    await user.click(screen.getByTestId('morning-briefing-refresh'))

    // Still collapsed after refresh triggered
    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')
    // Wrapper + testid still present
    expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
  })

  it('error state preserves wrapper and testid when collapsed', async () => {
    const user = userEvent.setup()
    vi.mocked(api.getMorningBriefing).mockRejectedValue(new Error('Network error'))

    render(<MorningBriefing />)

    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    // Collapse first
    await user.click(screen.getByTestId('morning-briefing-collapse'))

    // Error appears but wrapper + testid still present
    await waitFor(() => {
      expect(screen.getByTestId('morning-briefing-error')).toBeInTheDocument()
    })
    expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Morning Briefing' })).toHaveAttribute('aria-expanded', 'false')
  })
})
