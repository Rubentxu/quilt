// ─── DecayMonitor component tests ────────────────────────────────────────────
//
// CG-7: Decay Monitor end-to-end.

import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { DecayMonitor } from '../DecayMonitor'
import { api } from '@core/api-client'

// ─── Mock API ────────────────────────────────────────────────────────────────

const mockDto = {
  alerts: [
    {
      blockId: 'block-3',
      contentPreview: 'This block has not been updated in 30 days',
      pageName: 'journals/2024-01-01',
      daysSinceUpdate: 45,
      severity: 'high',
      reason: 'No updates in 45 days — significantly stale',
    },
    {
      blockId: 'block-4',
      contentPreview: 'This block has not been updated in 15 days',
      pageName: 'journals/2024-01-10',
      daysSinceUpdate: 20,
      severity: 'medium',
      reason: 'No updates in 20 days — consider reviewing',
    },
  ],
  totalAlerts: 2,
  countsBySeverity: { low: 0, medium: 1, high: 1 },
  generatedAt: new Date().toISOString(),
}

const mockEmptyDto = {
  alerts: [],
  totalAlerts: 0,
  countsBySeverity: { low: 0, medium: 0, high: 0 },
  generatedAt: new Date().toISOString(),
}

vi.mock('@core/api-client', () => ({
  api: {
    getDecayAlerts: vi.fn(),
  },
}))

describe('DecayMonitor', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── Loading state ─────────────────────────────────────────────────────

  it('shows loading state initially', async () => {
    vi.mocked(api.getDecayAlerts).mockImplementation(
      () => new Promise(() => {}), // never resolves
    )

    render(<DecayMonitor />)

    expect(screen.getByTestId('decay-monitor-loading')).toBeInTheDocument()
    expect(screen.getByText('Loading decay…')).toBeInTheDocument()
  })

  // ─── Error state ───────────────────────────────────────────────────────

  it('shows error state when API fails', async () => {
    vi.mocked(api.getDecayAlerts).mockRejectedValue(new Error('Network error'))

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-error')).toBeInTheDocument()
    })
    expect(screen.getByText('Network error')).toBeInTheDocument()
  })

  // ─── Empty state ───────────────────────────────────────────────────────

  it('shows empty state when no alerts', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockEmptyDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor')).toBeInTheDocument()
    })

    expect(screen.getByTestId('decay-monitor-empty')).toBeInTheDocument()
    expect(
      screen.getByText('No decay alerts — everything looks healthy'),
    ).toBeInTheDocument()
  })

  // ─── With data state ───────────────────────────────────────────────────

  it('renders with-data state with grouped lists', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor')).toBeInTheDocument()
    })

    expect(screen.getByTestId('decay-monitor-group-high')).toBeInTheDocument()
    expect(screen.getByTestId('decay-monitor-group-medium')).toBeInTheDocument()
    expect(screen.getByTestId('decay-monitor-item-block-3')).toBeInTheDocument()
    expect(screen.getByTestId('decay-monitor-item-block-4')).toBeInTheDocument()
  })

  // ─── Severity grouping ─────────────────────────────────────────────────

  it('renders high-severity alert with correct text', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-item-block-3')).toBeInTheDocument()
    })

    const item = screen.getByTestId('decay-monitor-item-block-3')
    expect(item.textContent).toContain('45d ago')
  })

  it('renders medium-severity alert with correct text', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-item-block-4')).toBeInTheDocument()
    })

    const item = screen.getByTestId('decay-monitor-item-block-4')
    expect(item.textContent).toContain('20d ago')
  })

  // ─── Click / keyboard navigation ──────────────────────────────────────

  it('clicking an alert calls onNavigate', async () => {
    const onNavigate = vi.fn()
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor onNavigate={onNavigate} />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-item-block-3')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('decay-monitor-item-block-3'))

    expect(onNavigate).toHaveBeenCalledWith('block-3', 'journals/2024-01-01')
  })

  it('pressing Enter on a focused alert calls onNavigate', async () => {
    const onNavigate = vi.fn()
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor onNavigate={onNavigate} />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-item-block-3')).toBeInTheDocument()
    })

    const item = screen.getByTestId('decay-monitor-item-block-3') as HTMLElement
    item.focus()
    const user = userEvent.setup()
    await user.keyboard('{Enter}')

    expect(onNavigate).toHaveBeenCalledWith('block-3', 'journals/2024-01-01')
  })

  // ─── Refresh button ────────────────────────────────────────────────────

  it('refresh button re-fetches data', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor')).toBeInTheDocument()
    })

    const user = userEvent.setup()
    await user.click(screen.getByTestId('decay-monitor-refresh'))

    expect(api.getDecayAlerts).toHaveBeenCalledTimes(2)
  })

  // ─── Header counts ─────────────────────────────────────────────────────

  it('header shows per-severity counts', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-counts')).toBeInTheDocument()
    })

    const counts = screen.getByTestId('decay-monitor-counts')
    expect(counts.textContent).toContain('1 high')
    expect(counts.textContent).toContain('1 medium')
    expect(counts.textContent).toContain('0 low')
  })

  // ─── Accessibility ─────────────────────────────────────────────────────

  it('has the Decay Monitor region landmark', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Decay Monitor' })).toBeInTheDocument()
    })
  })

  it('shows generated timestamp in footer', async () => {
    vi.mocked(api.getDecayAlerts).mockResolvedValue(mockDto)

    render(<DecayMonitor />)

    await waitFor(() => {
      expect(screen.getByTestId('decay-monitor-footer')).toBeInTheDocument()
    })
    expect(screen.getByTestId('decay-monitor-footer').textContent).toContain(
      'Generated',
    )
  })
})
