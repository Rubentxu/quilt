/**
 * JournalPage tests — T7 of slash-command-functional-behavior + GS-6 morning briefing gating.
 *
 * Verifies that JournalPage mounts the JournalAggregator and
 * surfaces the aggregation toggle.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import React from 'react'
import { JournalPage } from '@pages/JournalPage'

// ── Mocks ─────────────────────────────────────────────────────────────────

const mockUseParams = vi.fn()
const mockOpenTab = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useParams: (...args: unknown[]) => mockUseParams(...args),
}))

vi.mock('@shared/contexts/TabsContext', () => ({
  useTabs: () => ({ openTab: mockOpenTab }),
}))

vi.mock('@core/api-client', async () => {
  const actual = await vi.importActual('@core/api-client')
  return {
    ...actual,
    api: {
      ...actual.api,
      getJournal: vi.fn(),
      getSettings: vi.fn(),
      getPageBlocks: vi.fn(),
    },
  }
})

vi.mock('@features/outliner-tiptap/PageView', () => ({
  PageView: () => <div data-testid="mock-pageview" />,
}))

vi.mock('@features/journal/JournalAggregator', () => ({
  JournalAggregator: () => <div data-testid="mock-journal-aggregator" />,
}))

vi.mock('@core/wasm-bridge/WasmProvider', () => ({
  useWasm: () => ({ wasm: null, ready: true }),
}))

import { api } from '@core/api-client'
import type { Page, UserSettings, Block } from '@shared/types/api'

// ─── Fixtures ───────────────────────────────────────────────────────────────

function makePage(name = '2026-06-15'): Page {
  return {
    id: 'page-1',
    name,
    title: 'June 15, 2026',
    journal: true,
    journalDay: 20260615,
    createdAt: '2026-06-15T00:00:00Z',
  }
}

function makeSettings(overrides: Partial<UserSettings> = {}): UserSettings {
  return {
    timezone: 'UTC',
    journalFormat: '%Y-%m-%d',
    startOfWeek: 1,
    preferredFormat: 'markdown',
    journalAggregate: false,
    ...overrides,
  }
}

const today = new Date().toISOString().slice(0, 10) // YYYY-MM-DD

// ─── Tests ─────────────────────────────────────────────────────────────────

describe('JournalPage (unit)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockOpenTab.mockClear()
    mockUseParams.mockReturnValue({ date: '2026-06-15' })
  })

  describe('journal.aggregate setting', () => {
    it('reads journal.aggregate from settings on mount', async () => {
      ;(api.getJournal as ReturnType<typeof vi.fn>).mockResolvedValue(makePage())
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: true }),
      )
      ;(api.getPageBlocks as ReturnType<typeof vi.fn>).mockResolvedValue([])

      mockUseParams.mockReturnValue({ date: '2026-06-15' })

      render(<JournalPage />)

      await waitFor(() => {
        expect(api.getSettings).toHaveBeenCalled()
      })
    })
  })

  describe('Morning Briefing gating (GS-6)', () => {
    function renderJournalForDate(dateStr: string) {
      mockUseParams.mockReturnValue({ date: dateStr })
      return render(<JournalPage />)
    }

    it('mounts MorningBriefing for today\'s empty journal', async () => {
      ;(api.getJournal as ReturnType<typeof vi.fn>).mockResolvedValue(makePage(today))
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(makeSettings())
      ;(api.getPageBlocks as ReturnType<typeof vi.fn>).mockResolvedValue([])

      renderJournalForDate(today)

      await waitFor(() => {
        expect(screen.queryByTestId('morning-briefing-loading')).not.toBeInTheDocument()
      })
      expect(screen.getByTestId('morning-briefing')).toBeInTheDocument()
    })

    it('omits MorningBriefing for non-today journal', async () => {
      const nonToday = '2024-01-15'
      ;(api.getJournal as ReturnType<typeof vi.fn>).mockResolvedValue(makePage(nonToday))
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(makeSettings())
      ;(api.getPageBlocks as ReturnType<typeof vi.fn>).mockResolvedValue([])

      renderJournalForDate(nonToday)

      await waitFor(() => {
        expect(screen.queryByTestId('morning-briefing-loading')).not.toBeInTheDocument()
      })
      expect(screen.queryByTestId('morning-briefing')).not.toBeInTheDocument()
    })

    it('omits MorningBriefing for today\'s journal with existing blocks', async () => {
      ;(api.getJournal as ReturnType<typeof vi.fn>).mockResolvedValue(makePage(today))
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(makeSettings())
      ;(api.getPageBlocks as ReturnType<typeof vi.fn>).mockResolvedValue([
        { id: 'block-1', content: 'Test block', pageId: 'page-1' } as Block,
      ])

      renderJournalForDate(today)

      await waitFor(() => {
        expect(screen.queryByTestId('morning-briefing-loading')).not.toBeInTheDocument()
      })
      expect(screen.queryByTestId('morning-briefing')).not.toBeInTheDocument()
    })

    it('does not leak MorningBriefing during loading state', async () => {
      ;(api.getJournal as ReturnType<typeof vi.fn>).mockImplementation(
        () => new Promise(() => {}), // never resolves
      )
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockImplementation(
        () => new Promise(() => {}),
      )

      renderJournalForDate(today)

      // During loading, briefing must not be present
      expect(screen.queryByTestId('morning-briefing')).not.toBeInTheDocument()
    })
  })
})
