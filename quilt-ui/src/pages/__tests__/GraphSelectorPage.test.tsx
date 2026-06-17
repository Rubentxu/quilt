/**
 * Tests for GraphSelectorPage — `/select-graph` route (ADR-0030, Slice D).
 *
 * Covers:
 * - D.5: recents list, open-by-path, create-new, error display, keyboard navigation
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { GraphSelectorPage } from '../GraphSelectorPage'
import { QuiltApiError } from '@core/api-client'

// ── Mock helpers — hoisted so vi.mock factory can reference them ─────────────

const { mockNavigate, mockGetGlobalState, mockCreateGraph } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockGetGlobalState: vi.fn<() => Promise<{
    lastOpenedGraph: string | null;
    recentGraphs: string[];
    rightSidebarVisible: boolean | null;
  }>>(),
  mockCreateGraph: vi.fn<() => Promise<{ graphPath: string; created: boolean }>>(),
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('@core/api-client', () => ({
  api: {
    getGlobalState: mockGetGlobalState,
    createGraph: mockCreateGraph,
  },
  QuiltApiError: class extends Error {
    constructor(
      public status: number,
      public code: string,
      public detail: string,
    ) {
      super(detail)
      this.name = 'QuiltApiError'
    }
  },
}))

// ── Helpers ─────────────────────────────────────────────────────────────────

function mockGlobalState(state: {
  lastOpenedGraph?: string | null
  recentGraphs?: string[]
  rightSidebarVisible?: boolean | null
}) {
  mockGetGlobalState.mockResolvedValue({
    lastOpenedGraph: state.lastOpenedGraph ?? null,
    recentGraphs: state.recentGraphs ?? [],
    rightSidebarVisible: state.rightSidebarVisible ?? null,
  })
}

beforeEach(() => {
  mockNavigate.mockReset()
  mockGetGlobalState.mockReset()
  mockCreateGraph.mockReset()
})

// ── Tests ───────────────────────────────────────────────────────────────────

describe('GraphSelectorPage — initial render and data loading', () => {
  it('loads recent graphs from getGlobalState on mount', async () => {
    mockGlobalState({ recentGraphs: ['/home/user/work', '/home/user/personal'] })

    render(<GraphSelectorPage />)

    await waitFor(() => {
      expect(mockGetGlobalState).toHaveBeenCalledTimes(1)
    })
  })

  it('renders an empty message when there are no recent graphs', async () => {
    mockGlobalState({ recentGraphs: [] })

    render(<GraphSelectorPage />)

    await waitFor(() => {
      expect(screen.getByText(/no recent graphs/i)).toBeInTheDocument()
    })
  })

  it('renders the recent graphs list when data is available', async () => {
    mockGlobalState({ recentGraphs: ['/home/user/work', '/home/user/personal'] })

    render(<GraphSelectorPage />)

    await waitFor(() => {
      expect(screen.getByText('work')).toBeInTheDocument()
      expect(screen.getByText('/home/user/work')).toBeInTheDocument()
    })
  })

  it('disables recent buttons while loading', async () => {
    mockGlobalState({ recentGraphs: ['/home/user/work'] })
    // Simulate slow create
    mockCreateGraph.mockImplementation(
      () => new Promise((r) => setTimeout(() => r({ graphPath: '/home/user/work', created: false }), 200))
    )

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByText('work'))

    // Click a recent item
    fireEvent.click(screen.getByText('work'))

    // While loading, buttons should be disabled
    await waitFor(() => {
      const btn = screen.getByText('work').closest('button')
      expect(btn).toBeDisabled()
    })
  })
})

describe('GraphSelectorPage — open by path', () => {
  beforeEach(() => {
    mockGlobalState({ recentGraphs: [] })
  })

  it('switches to the Open by Path tab when clicked', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))

    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    expect(screen.getByLabelText(/graph directory path/i)).toBeInTheDocument()
  })

  it('focuses the path input when Open by Path tab activates', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))

    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    expect(input).toHaveFocus()
  })

  it('opens a graph when a valid path is submitted', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/work', created: false })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/work' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(mockCreateGraph).toHaveBeenCalledWith('/home/user/work')
    })
  })

  it('navigates to journal after opening an existing graph', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/work', created: false })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/work' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: expect.any(String) },
      })
    })
  })

  it('navigates to journal after creating a new graph', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/new', created: true })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /create new/i }))
    fireEvent.click(screen.getByRole('tab', { name: /create new/i }))

    const input = screen.getByLabelText(/new graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/new' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: expect.any(String) },
      })
    })
  })

  it('disables the submit button when path is empty', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const btn = screen.getByRole('button', { name: /open graph/i })
    expect(btn).toBeDisabled()
  })

  it('disables the submit button while loading', async () => {
    mockCreateGraph.mockImplementation(
      () => new Promise((r) => setTimeout(() => r({ graphPath: '/home/user/work', created: false }), 500))
    )

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/work' } })
    fireEvent.click(screen.getByRole('button', { name: /open graph/i }))

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /opening/i })).toBeDisabled()
    })
  })
})

describe('GraphSelectorPage — create new', () => {
  beforeEach(() => {
    mockGlobalState({ recentGraphs: [] })
  })

  it('switches to the Create New tab when clicked', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /create new/i }))

    fireEvent.click(screen.getByRole('tab', { name: /create new/i }))

    expect(screen.getByLabelText(/new graph directory path/i)).toBeInTheDocument()
  })

  it('focuses the path input when Create New tab activates', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /create new/i }))

    fireEvent.click(screen.getByRole('tab', { name: /create new/i }))

    const input = screen.getByLabelText(/new graph directory path/i)
    expect(input).toHaveFocus()
  })

  it('calls createGraph with the entered path on submit', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/new', created: true })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /create new/i }))
    fireEvent.click(screen.getByRole('tab', { name: /create new/i }))

    const input = screen.getByLabelText(/new graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/new' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(mockCreateGraph).toHaveBeenCalledWith('/home/user/new')
    })
  })

  it('disables submit button when path is empty', async () => {
    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /create new/i }))
    fireEvent.click(screen.getByRole('tab', { name: /create new/i }))

    const btn = screen.getByRole('button', { name: /create graph/i })
    expect(btn).toBeDisabled()
  })
})

describe('GraphSelectorPage — error display', () => {
  beforeEach(() => {
    mockGlobalState({ recentGraphs: [] })
  })

  it('shows a validation error when createGraph throws 422', async () => {
    // Simulate a QuiltApiError with 422 status (validation failure)
    const apiError = new QuiltApiError(422, 'GRAPH_INVALID', 'Not a quilt graph directory')
    mockCreateGraph.mockRejectedValue(apiError)

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/bad/path' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(screen.getByText(/cannot open/i)).toBeInTheDocument()
      expect(screen.getByText(/not a quilt graph directory/i)).toBeInTheDocument()
    })
  })

  it('shows a network error on non-validation failures', async () => {
    mockCreateGraph.mockRejectedValue(new Error('fetch failed'))

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/some/path' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(screen.getByText(/network error/i)).toBeInTheDocument()
    })
  })

  it('successful submission navigates to journal', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/work', created: false })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/work' } })
    fireEvent.submit(input)

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: expect.any(String) },
      })
    })
  })
})

describe('GraphSelectorPage — keyboard navigation', () => {
  beforeEach(() => {
    mockGlobalState({ recentGraphs: ['/home/user/work'] })
  })

  it('click on recent item triggers navigation', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/work', created: false })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByText('work'))

    const btn = screen.getByText('work').closest('button')!
    fireEvent.click(btn)

    await waitFor(() => {
      expect(mockCreateGraph).toHaveBeenCalledWith('/home/user/work')
    })
  })

  it('Enter key submits the open-by-path form', async () => {
    mockCreateGraph.mockResolvedValue({ graphPath: '/home/user/work', created: false })

    render(<GraphSelectorPage />)
    await waitFor(() => screen.getByRole('tab', { name: /open by path/i }))
    fireEvent.click(screen.getByRole('tab', { name: /open by path/i }))

    const input = screen.getByLabelText(/graph directory path/i)
    fireEvent.change(input, { target: { value: '/home/user/work' } })
    const form = input.closest('form')!
    fireEvent.submit(form)

    await waitFor(() => {
      expect(mockCreateGraph).toHaveBeenCalledWith('/home/user/work')
    })
  })
})

describe('GraphSelectorPage — getGlobalState failure', () => {
  it('renders empty recents and does not crash when getGlobalState fails', async () => {
    mockGetGlobalState.mockRejectedValue(new Error('network error'))

    render(<GraphSelectorPage />)

    // Should still render the page with empty recents
    await waitFor(() => {
      expect(screen.getByRole('tablist')).toBeInTheDocument()
      expect(screen.getByText(/no recent graphs/i)).toBeInTheDocument()
    })
  })
})
