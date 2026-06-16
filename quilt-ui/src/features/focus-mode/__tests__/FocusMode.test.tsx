// ─── FocusMode component tests ──────────────────────────────────────
//
// Tests for FocusModeContext, FocusModeToggle, FocusModeLayout, and AIPanel.

import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import React from 'react'

// ─── Mocks ───────────────────────────────────────────────────────────

// Mock api client
vi.mock('@core/api-client', () => ({
  api: {
    getDecayAlerts: vi.fn().mockResolvedValue({
      alerts: [
        {
          block_id: 'block-1',
          block_content_preview: 'This block has not been updated in 30 days',
          days_since_update: 30,
          severity: 'high',
        },
      ],
      total_alerts: 1,
      counts_by_severity: { low: 0, medium: 0, high: 1 },
      generated_at: new Date().toISOString(),
    }),
    getSerendipity: vi.fn().mockResolvedValue({
      highlights: [
        {
          block_a_id: 'a1',
          block_b_id: 'b1',
          block_a_preview: 'Block A content',
          block_b_preview: 'Block B content',
          explanation: 'These blocks share a common reference to project X',
          confidence: 0.85,
        },
      ],
      total: 1,
      generated_at: new Date().toISOString(),
    }),
    spawnAgent: vi.fn().mockResolvedValue({
      id: 'agent-123',
      agent_type: 'decay-annotator',
      status: 'Queued',
      context_page: 'test/page',
      summary: null,
      blocks_modified: 0,
      started_at: null,
      completed_at: null,
      error: null,
    }),
    getAgent: vi.fn().mockResolvedValue({
      id: 'agent-123',
      agent_type: 'decay-annotator',
      status: 'Completed',
      context_page: 'test/page',
      summary: 'Found 3 blocks to update',
      blocks_modified: 3,
      started_at: new Date().toISOString(),
      completed_at: new Date().toISOString(),
      error: null,
    }),
  },
}))

// ─── FocusModeContext tests ──────────────────────────────────────────

import { FocusModeProvider, useFocusMode } from '../FocusModeContext'

function ConsumerComponent() {
  const { isActive, toggle, setActive, isAIPanelOpen, toggleAIPanel, setAIPanelOpen } = useFocusMode()
  return (
    <div>
      <span data-testid="is-active">{String(isActive)}</span>
      <span data-testid="is-ai-panel-open">{String(isAIPanelOpen)}</span>
      <button data-testid="toggle" onClick={toggle}>Toggle</button>
      <button data-testid="set-true" onClick={() => setActive(true)}>Set True</button>
      <button data-testid="set-false" onClick={() => setActive(false)}>Set False</button>
      <button data-testid="toggle-ai" onClick={toggleAIPanel}>Toggle AI</button>
      <button data-testid="set-ai-open" onClick={() => setAIPanelOpen(true)}>Open AI</button>
      <button data-testid="set-ai-closed" onClick={() => setAIPanelOpen(false)}>Close AI</button>
    </div>
  )
}

describe('FocusModeContext', () => {
  it('initializes with isActive false and isAIPanelOpen false', () => {
    render(
      <FocusModeProvider>
        <ConsumerComponent />
      </FocusModeProvider>
    )
    expect(screen.getByTestId('is-active').textContent).toBe('false')
    expect(screen.getByTestId('is-ai-panel-open').textContent).toBe('false')
  })

  it('toggle flips isActive', () => {
    render(
      <FocusModeProvider>
        <ConsumerComponent />
      </FocusModeProvider>
    )
    fireEvent.click(screen.getByTestId('toggle'))
    expect(screen.getByTestId('is-active').textContent).toBe('true')
    fireEvent.click(screen.getByTestId('toggle'))
    expect(screen.getByTestId('is-active').textContent).toBe('false')
  })

  it('setActive sets isActive explicitly', () => {
    render(
      <FocusModeProvider>
        <ConsumerComponent />
      </FocusModeProvider>
    )
    fireEvent.click(screen.getByTestId('set-true'))
    expect(screen.getByTestId('is-active').textContent).toBe('true')
    fireEvent.click(screen.getByTestId('set-false'))
    expect(screen.getByTestId('is-active').textContent).toBe('false')
  })

  it('toggleAIPanel flips isAIPanelOpen', () => {
    render(
      <FocusModeProvider>
        <ConsumerComponent />
      </FocusModeProvider>
    )
    fireEvent.click(screen.getByTestId('toggle-ai'))
    expect(screen.getByTestId('is-ai-panel-open').textContent).toBe('true')
    fireEvent.click(screen.getByTestId('toggle-ai'))
    expect(screen.getByTestId('is-ai-panel-open').textContent).toBe('false')
  })

  it('setAIPanelOpen sets isAIPanelOpen explicitly', () => {
    render(
      <FocusModeProvider>
        <ConsumerComponent />
      </FocusModeProvider>
    )
    fireEvent.click(screen.getByTestId('set-ai-open'))
    expect(screen.getByTestId('is-ai-panel-open').textContent).toBe('true')
    fireEvent.click(screen.getByTestId('set-ai-closed'))
    expect(screen.getByTestId('is-ai-panel-open').textContent).toBe('false')
  })
})

// ─── FocusModeLayout tests ───────────────────────────────────────────

import { FocusModeLayout } from '../FocusModeLayout'

describe('FocusModeLayout', () => {
  it('renders children directly when focus mode is inactive', () => {
    render(
      <FocusModeProvider>
        <FocusModeLayout>
          <div data-testid="child">Editor content</div>
        </FocusModeLayout>
      </FocusModeProvider>
    )
    expect(screen.getByTestId('child').textContent).toBe('Editor content')
    expect(screen.queryByTestId('focus-mode-layout')).toBeNull()
  })

  it('applies focus mode layout when active', () => {
    const { rerender } = render(
      <FocusModeProvider>
        <FocusModeLayout>
          <div data-testid="child">Editor content</div>
        </FocusModeLayout>
      </FocusModeProvider>
    )

    // Activate focus mode
    const ToggleComponent = () => {
      const { setActive } = useFocusMode()
      return <button data-testid="activate" onClick={() => setActive(true)}>Activate</button>
    }

    rerender(
      <FocusModeProvider>
        <ToggleComponent />
        <FocusModeLayout>
          <div data-testid="child">Editor content</div>
        </FocusModeLayout>
      </FocusModeProvider>
    )

    fireEvent.click(screen.getByTestId('activate'))

    expect(screen.getByTestId('focus-mode-layout')).toBeTruthy()
    expect(screen.getByTestId('focus-mode-editor')).toBeTruthy()
  })

  it('shows AI panel when isAIPanelOpen is true in focus mode', () => {
    const { rerender } = render(
      <FocusModeProvider>
        <FocusModeLayout>
          <div data-testid="child">Editor content</div>
        </FocusModeLayout>
      </FocusModeProvider>
    )

    const ToggleAIComponent = () => {
      const { setActive, setAIPanelOpen } = useFocusMode()
      return (
        <button
          data-testid="activate"
          onClick={() => {
            setActive(true)
            setAIPanelOpen(true)
          }}
        >
          Activate
        </button>
      )
    }

    rerender(
      <FocusModeProvider>
        <ToggleAIComponent />
        <FocusModeLayout>
          <div data-testid="child">Editor content</div>
        </FocusModeLayout>
      </FocusModeProvider>
    )

    fireEvent.click(screen.getByTestId('activate'))

    expect(screen.getByTestId('focus-mode-layout')).toBeTruthy()
    expect(screen.getByTestId('ai-panel')).toBeTruthy()
  })
})

// ─── AIPanel tests ──────────────────────────────────────────────────

import { AIPanel } from '../AIPanel'

describe('AIPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders AI panel with close button', () => {
    render(
      <FocusModeProvider>
        <AIPanel pageName="test/page" />
      </FocusModeProvider>
    )
    expect(screen.getByTestId('ai-panel')).toBeTruthy()
    expect(screen.getByTestId('ai-panel-close')).toBeTruthy()
  })

  it('renders Ask AI input field', () => {
    render(
      <FocusModeProvider>
        <AIPanel pageName="test/page" />
      </FocusModeProvider>
    )
    expect(screen.getByTestId('ask-ai-input')).toBeTruthy()
  })

  it('renders insight cards for decay and serendipity', async () => {
    render(
      <FocusModeProvider>
        <AIPanel pageName="test/page" />
      </FocusModeProvider>
    )

    // Wait for async insights to load
    await waitFor(() => {
      expect(screen.getAllByTestId('insight-card').length).toBeGreaterThan(0)
    })
  })

  it('shows loading state for decay insights initially', () => {
    render(
      <FocusModeProvider>
        <AIPanel pageName="test/page" />
      </FocusModeProvider>
    )
    // Both decay and serendipity show "Loading..." initially
    const loadingElements = screen.getAllByText('Loading...')
    expect(loadingElements.length).toBeGreaterThan(0)
  })

  it('submits question to spawn agent', async () => {
    const { api } = await import('@core/api-client')
    render(
      <FocusModeProvider>
        <AIPanel pageName="test/page" />
      </FocusModeProvider>
    )

    const input = screen.getByTestId('ask-ai-input')
    fireEvent.change(input, { target: { value: 'What should I work on today?' } })

    const submitButton = screen.getByTestId('ask-ai-submit')
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(api.spawnAgent).toHaveBeenCalledWith({
        agent_type: 'decay-annotator',
        context_page: 'test/page',
        model: null,
        queue_mode: null,
      })
    })
  })
})
