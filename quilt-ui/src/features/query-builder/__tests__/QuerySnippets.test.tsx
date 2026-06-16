/**
 * Tests for QuerySnippets component.
 */

import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import userEvent from '@testing-library/user-event'
import { QuerySnippets, QUERY_SNIPPETS } from '../QuerySnippets'

describe('QuerySnippets', () => {
  const onInsert = vi.fn()

  beforeEach(() => {
    onInsert.mockClear()
  })

  it('renders all snippet categories', () => {
    render(<QuerySnippets onInsert={onInsert} />)
    expect(screen.getByTestId('query-snippets')).toBeInTheDocument()
    // Should have journal category
    expect(screen.getByText('Journal')).toBeInTheDocument()
    expect(screen.getByText('Tasks')).toBeInTheDocument()
    expect(screen.getByText('Pages')).toBeInTheDocument()
  })

  it('renders all snippet rows', () => {
    render(<QuerySnippets onInsert={onInsert} />)
    // Should have all snippets rendered
    expect(screen.getByTestId('snippet-row-journal-this-week')).toBeInTheDocument()
    expect(screen.getByTestId('snippet-row-tasks-scheduled-today')).toBeInTheDocument()
    expect(screen.getByTestId('snippet-row-tasks-todo')).toBeInTheDocument()
    expect(screen.getByTestId('snippet-row-tasks-overdue')).toBeInTheDocument()
    expect(screen.getByTestId('snippet-row-tasks-in-progress')).toBeInTheDocument()
  })

  it('calls onInsert with DSL when a snippet row is clicked', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-row-tasks-todo'))
    expect(onInsert).toHaveBeenCalledTimes(1)
    expect(onInsert).toHaveBeenCalledWith('(task todo)')
  })

  it('calls onInsert with correct DSL for scheduled today', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-row-tasks-scheduled-today'))
    expect(onInsert).toHaveBeenCalledWith('(scheduled today)')
  })

  it('calls onInsert with journal this week DSL', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-row-journal-this-week'))
    expect(onInsert).toHaveBeenCalledWith("(temporal :this-week (page \"{{page}}\"))")
  })

  it('calls onInsert with journal today DSL', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-row-journal-today'))
    expect(onInsert).toHaveBeenCalledWith("(temporal :today (page \"{{page}}\"))")
  })

  it('calls onInsert with page by name DSL', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-row-page-by-name'))
    expect(onInsert).toHaveBeenCalledWith("(page \"{{page-name}}\")")
  })

  it('renders copy button on each snippet', () => {
    render(<QuerySnippets onInsert={onInsert} />)
    expect(screen.getByTestId('snippet-copy-tasks-todo')).toBeInTheDocument()
    expect(screen.getByTestId('snippet-copy-journal-this-week')).toBeInTheDocument()
  })

  it('does not insert DSL when copy button is clicked (only copies)', async () => {
    const user = userEvent.setup()
    render(<QuerySnippets onInsert={onInsert} />)

    await user.click(screen.getByTestId('snippet-copy-tasks-todo'))
    // Copy should not trigger insert
    expect(onInsert).not.toHaveBeenCalled()
  })
})
