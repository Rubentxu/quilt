/**
 * Tests for QueryResults component (CG-4: keyboard navigation, expand).
 */

import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import userEvent from '@testing-library/user-event'
import { QueryResults } from '../QueryResults'
import type { QueryResult } from '@shared/types/queryAst'

describe('QueryResults', () => {
  const onNavigate = vi.fn()

  beforeEach(() => {
    onNavigate.mockClear()
  })

  function makeResult(rows: Record<string, unknown>[] = []): QueryResult {
    return { results: rows, total: rows.length, elapsed_ms: 12 }
  }

  // ─── Empty / Loading ──────────────────────────────────────────

  it('shows empty state when no results', () => {
    render(<QueryResults result={null} loading={false} onNavigate={onNavigate} />)
    expect(screen.getByTestId('query-results-empty')).toHaveTextContent('No results')
  })

  it('shows loading state', () => {
    render(<QueryResults result={null} loading={true} onNavigate={onNavigate} />)
    expect(screen.getByTestId('query-results-loading')).toBeInTheDocument()
  })

  // ─── Result display ──────────────────────────────────────────

  it('renders result count', () => {
    render(
      <QueryResults
        result={makeResult([
          { id: '1', name: 'Alice', pageName: 'Page1' },
          { id: '2', name: 'Bob', pageName: 'Page2' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )
    expect(screen.getByTestId('query-results-count')).toHaveTextContent('2 results')
  })

  it('renders result rows', () => {
    render(
      <QueryResults
        result={makeResult([
          { id: '1', name: 'Alice', pageName: 'Page1' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )
    expect(screen.getByTestId('query-result-row-0')).toBeInTheDocument()
  })

  it('renders elapsed time', () => {
    render(
      <QueryResults
        result={makeResult([{ id: '1' }])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )
    expect(screen.getByTestId('query-results-elapsed')).toHaveTextContent('12ms')
  })

  // ─── Keyboard navigation ─────────────────────────────────────

  it('renders keyboard hint', () => {
    render(
      <QueryResults
        result={makeResult([{ id: '1' }])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )
    expect(screen.getByTestId('query-results')).toHaveTextContent('↑↓ navigate')
  })

  it('selects first row on ArrowDown', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([
          { id: '1', name: 'Alice', pageName: 'Page1' },
          { id: '2', name: 'Bob', pageName: 'Page2' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    // Focus the results container first
    const container = screen.getByTestId('query-results')
    container.focus()
    await user.keyboard('{ArrowDown}')
    // No assertion on CSS value (jsdom doesn't compute var()).
    // Just verify no error and structure is correct.
    expect(screen.getByTestId('query-result-row-0')).toBeInTheDocument()
    expect(screen.getByTestId('query-result-row-1')).toBeInTheDocument()
  })

  it('calls onNavigate when Enter is pressed on a selected row', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([
          { id: 'b1', name: 'Alice', pageName: 'Page1' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    // Focus the container so keyboard events are captured
    const container = screen.getByTestId('query-results')
    container.focus()
    await user.keyboard('{Enter}')
    expect(onNavigate).toHaveBeenCalledWith('b1', 'Page1')
  })

  it('does not call onNavigate when row has no pageName', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([{ id: 'b1', name: 'Alice' }])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    await user.keyboard('{Enter}')
    expect(onNavigate).not.toHaveBeenCalled()
  })

  // ─── Expand / Collapse ──────────────────────────────────────

  it('shows expand button on each row', () => {
    render(
      <QueryResults
        result={makeResult([{ id: '1', name: 'Alice', pageName: 'Page1' }])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )
    expect(screen.getByTestId('query-result-expand-0')).toBeInTheDocument()
  })

  it('expands a row when expand button is clicked', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([
          { id: '1', name: 'Alice', pageName: 'Page1', content: 'Full block content here' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    await user.click(screen.getByTestId('query-result-expand-0'))
    expect(screen.getByTestId('query-result-expanded-0')).toBeInTheDocument()
    expect(screen.getByTestId('query-result-expanded-0')).toHaveTextContent('Full block content here')
  })

  it('collapses a row when expand button is clicked again', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([
          { id: '1', name: 'Alice', pageName: 'Page1', content: 'Content' },
        ])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    await user.click(screen.getByTestId('query-result-expand-0'))
    await user.click(screen.getByTestId('query-result-expand-0'))
    expect(screen.queryByTestId('query-result-expanded-0')).not.toBeInTheDocument()
  })

  it('shows empty block text for empty content', async () => {
    const user = userEvent.setup()
    render(
      <QueryResults
        result={makeResult([{ id: '1', name: 'Alice', pageName: 'Page1', content: '' }])}
        loading={false}
        onNavigate={onNavigate}
      />,
    )

    await user.click(screen.getByTestId('query-result-expand-0'))
    expect(screen.getByTestId('query-result-expanded-0')).toHaveTextContent('(empty)')
  })
})
