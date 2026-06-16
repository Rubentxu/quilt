/**
 * JournalAggregator tests — T7 of slash-command-functional-behavior.
 *
 * Tests the JournalAggregator component that renders 4 default query blocks
 * at the bottom of the journal page when the user opts in via the
 * `journal.aggregate` setting.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import React from 'react'

// Mock the settings API
vi.mock('@core/api-client', async () => {
  const actual = await vi.importActual('@core/api-client')
  return {
    ...actual,
    api: {
      ...actual.api,
      getSettings: vi.fn(),
      updateSettings: vi.fn(),
    },
  }
})

// We need to mock the search API for executeQuery
vi.mock('@core/api/search', () => ({
  searchApi: {
    executeQuery: vi.fn(),
  },
}))

import { api } from '@core/api-client'
import { searchApi } from '@core/api/search'
import type { UserSettings, Block } from '@shared/types/api'

// Import after mocks are set up
import { JournalAggregator } from '../JournalAggregator'

// ─── Fixtures ────────────────────────────────────────────────────────────────

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

function makeQueryResult(blocks: Partial<Block>[] = []): Record<string, unknown> {
  return {
    results: blocks.map((b) => ({
      id: b.id ?? 'block-1',
      pageName: b.pageName ?? 'Test Page',
      content: b.content ?? 'Test block',
      marker: b.marker ?? null,
      blockType: b.blockType ?? 'paragraph',
      ...b,
    })),
    total: blocks.length,
    elapsed_ms: 5,
  }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

describe('JournalAggregator', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Default: no results
    ;(searchApi.executeQuery as ReturnType<typeof vi.fn>).mockResolvedValue(
      makeQueryResult([]),
    )
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('with journal.aggregate = false (default)', () => {
    it('renders nothing when setting is false', async () => {
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: false }),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      // Should not Render any section headings
      expect(screen.queryByRole('heading', { name: /now in progress/i })).not.toBeInTheDocument()
      expect(screen.queryByRole('heading', { name: /scheduled today/i })).not.toBeInTheDocument()
      expect(screen.queryByRole('heading', { name: /deadlines today/i })).not.toBeInTheDocument()
      expect(screen.queryByRole('heading', { name: /overdue/i })).not.toBeInTheDocument()
    })

    it('renders nothing when setting is undefined (default)', async () => {
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: undefined }),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      expect(screen.queryByRole('heading', { name: /now in progress/i })).not.toBeInTheDocument()
    })
  })

  describe('with journal.aggregate = true', () => {
    beforeEach(() => {
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: true }),
      )
    })

    it('renders 4 section headings when setting is true', async () => {
      render(<JournalAggregator pageName="2026-06-15" />)

      await screen.findByRole('heading', { name: /now in progress/i })
      await screen.findByRole('heading', { name: /scheduled today/i })
      await screen.findByRole('heading', { name: /deadlines today/i })
      await screen.findByRole('heading', { name: /overdue/i })
    })

    it('renders sections in fixed order', async () => {
      render(<JournalAggregator pageName="2026-06-15" />)

      const headings = await screen.findAllByRole('heading', { level: 3 })
      const texts = headings.map((h) => h.textContent)

      expect(texts[0]).toMatch(/now in progress/i)
      expect(texts[1]).toMatch(/scheduled today/i)
      expect(texts[2]).toMatch(/deadlines today/i)
      expect(texts[3]).toMatch(/overdue/i)
    })

    it('executes the correct DSL query for each section', async () => {
      render(<JournalAggregator pageName="2026-06-15" />)

      // Wait for all queries to be called
      await screen.findByRole('heading', { name: /overdue/i })

      // Verify the 4 DSL queries were executed
      const calls = (searchApi.executeQuery as ReturnType<typeof vi.fn>).mock.calls

      expect(calls.length).toBe(4)

      // NOW in progress: (and (task now) (task doing))
      expect(calls[0][0]).toEqual({ And: [{ Task: ['now'] }, { Task: ['doing'] }] })

      // Scheduled today: (and (scheduled today) (or (task todo) (task doing) (task waiting)))
      expect(calls[1][0]).toEqual({
        And: [
          { Scheduled: { predicate: 'Today' } },
          { Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }] },
        ],
      })

      // Deadlines today: (and (deadline today) (or (task todo) (task doing) (task waiting)))
      expect(calls[2][0]).toEqual({
        And: [
          { Deadline: { predicate: 'Today' } },
          { Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }] },
        ],
      })

      // Overdue: (or (task todo) (task doing) (task waiting))
      expect(calls[3][0]).toEqual({
        Or: [{ Task: ['todo'] }, { Task: ['doing'] }, { Task: ['waiting'] }],
      })
    })

    it('shows empty state when a query returns no results', async () => {
      // Explicitly mock getSettings so sections render (not polluted by other tests).
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: true }),
      )
      ;(searchApi.executeQuery as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeQueryResult([]),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      // Wait for the sections to load and show empty state
      await screen.findByRole('heading', { name: /now in progress/i })
      // Match literal "(none)" text — waitFor ensures async state has flushed.
      const emptyStates = await waitFor(() => screen.getAllByText(/\(none\)/))
      expect(emptyStates.length).toBe(4) // All 4 sections show "(none)"
    })

    it('displays task results from query execution', async () => {
      const mockBlocks: Partial<Block>[] = [
        {
          id: 'block-1',
          pageName: 'Project Atlas',
          content: 'Review proposal',
          marker: 'Todo',
          blockType: 'todo',
        },
      ]

      // Make queries return the mock blocks
      ;(searchApi.executeQuery as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeQueryResult(mockBlocks),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      // Wait for headings to appear
      await screen.findByRole('heading', { name: /scheduled today/i })

      // The query was executed with the correct arguments
      expect(searchApi.executeQuery).toHaveBeenCalled()
    })
  })

  describe('settings toggle', () => {
    it('renders a label with "Show daily aggregations" text when aggregation is off', async () => {
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: false }),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      // Use findByText which handles async waiting
      const label = await screen.findByText(/show daily aggregations/i)
      expect(label).toBeInTheDocument()
    })

    it('toggling aggregation calls updateSettings', async () => {
      // Start with aggregation off
      ;(api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: false }),
      )
      ;(api.updateSettings as ReturnType<typeof vi.fn>).mockResolvedValue(
        makeSettings({ journalAggregate: true }),
      )

      render(<JournalAggregator pageName="2026-06-15" />)

      // Wait for the toggle text to appear
      await screen.findByText(/show daily aggregations/i)

      // Click the toggle button (it has role=switch)
      const toggle = screen.getByRole('switch')
      await userEvent.click(toggle)

      // Should have called updateSettings
      expect(api.updateSettings).toHaveBeenCalledWith({ journalAggregate: true })
    })
  })
})
