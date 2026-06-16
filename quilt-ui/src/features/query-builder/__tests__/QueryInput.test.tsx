/**
 * Tests for QueryInput component (CG-4: error display, DSL mode).
 */

import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import userEvent from '@testing-library/user-event'
import { QueryInput } from '../QueryInput'

// Mock the validateQuery utility
vi.mock('@shared/utils/validateQuery', () => ({
  validateQuery: vi.fn(),
}))

import { validateQuery } from '@shared/utils/validateQuery'

const mockValidateQuery = validateQuery as ReturnType<typeof vi.fn>

describe('QueryInput', () => {
  const onChange = vi.fn()
  const onExecute = vi.fn()
  const onErrorChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    mockValidateQuery.mockResolvedValue({ valid: true, ast: null, error: null })
  })

  // ─── Rendering ────────────────────────────────────────────────

  it('renders DSL textarea by default', () => {
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )
    expect(screen.getByTestId('query-input-dsl')).toBeInTheDocument()
  })

  it('renders the mode toggle button', () => {
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )
    expect(screen.getByTestId('query-input-mode-toggle')).toBeInTheDocument()
  })

  it('renders the Run button', () => {
    render(
      <QueryInput
        value="(task todo)"
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )
    expect(screen.getByTestId('query-input-run')).toBeInTheDocument()
  })

  it('Run button is disabled when input is empty', () => {
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )
    expect(screen.getByTestId('query-input-run')).toBeDisabled()
  })

  // ─── Error display ────────────────────────────────────────────

  it('displays error message when validation fails', async () => {
    mockValidateQuery.mockResolvedValueOnce({
      valid: false,
      ast: null,
      error: 'Unknown expression: foo',
    })

    render(
      <QueryInput
        value="foo"
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )

    // Wait for debounced validation
    await waitFor(() => {
      expect(screen.getByTestId('query-input-error')).toBeInTheDocument()
    })
    expect(screen.getByTestId('query-input-error')).toHaveTextContent('Unknown expression: foo')
  })

  it('renders "Show in docs" link when error present', async () => {
    mockValidateQuery.mockResolvedValueOnce({
      valid: false,
      ast: null,
      error: 'Syntax error',
    })

    render(
      <QueryInput
        value="(invalid)"
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )

    await waitFor(() => {
      expect(screen.getByTestId('query-input-error-docs-link')).toBeInTheDocument()
    })
  })

  it('does not display error when validation passes', async () => {
    mockValidateQuery.mockResolvedValueOnce({ valid: true, ast: null, error: null })

    render(
      <QueryInput
        value="(task todo)"
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )

    await waitFor(() => {
      expect(screen.queryByTestId('query-input-error')).not.toBeInTheDocument()
    })
  })

  it('calls onErrorChange when error is set', async () => {
    // Simulate error being passed as a prop (server-side error)
    render(
      <QueryInput
        value="(task todo)"
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
        error={{ message: 'Syntax error' }}
      />,
    )

    expect(screen.getByTestId('query-input-error')).toHaveTextContent('Syntax error')
    expect(onErrorChange).not.toHaveBeenCalled() // prop error doesn't trigger onErrorChange
  })

  // ─── Mode toggle ───────────────────────────────────────────────

  it('switches from DSL to chips mode', async () => {
    const user = userEvent.setup()
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
        availableKeys={['status', 'priority']}
        chips={[]}
        onChipsChange={vi.fn()}
        onChipsApply={vi.fn()}
        initialMode="dsl"
      />,
    )

    await user.click(screen.getByTestId('query-input-mode-toggle'))
    // Should now show FilterChipGroup instead of textarea
    expect(screen.queryByTestId('query-input-dsl')).not.toBeInTheDocument()
  })

  // ─── Expand ───────────────────────────────────────────────────

  it('shows syntax hints when expand is clicked', async () => {
    const user = userEvent.setup()
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )

    await user.click(screen.getByTestId('query-input-expand'))
    expect(screen.getByTestId('query-input-hints')).toBeInTheDocument()
  })

  it('hides syntax hints when expand is clicked again', async () => {
    const user = userEvent.setup()
    render(
      <QueryInput
        value=""
        onChange={onChange}
        onExecute={onExecute}
        onErrorChange={onErrorChange}
      />,
    )

    await user.click(screen.getByTestId('query-input-expand'))
    await user.click(screen.getByTestId('query-input-expand'))
    expect(screen.queryByTestId('query-input-hints')).not.toBeInTheDocument()
  })
})
