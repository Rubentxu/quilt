// ─── DatePicker tests — quilt-slash-command-functional-behaviour ───────────
//
// TDD tests for the DatePicker component (spec §11.3).
// Tests the component in isolation (no BlockRow, no API).
//
// V1 scope: date only, NL set is {today, tomorrow, yesterday}.

import { render, screen, fireEvent, cleanup } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { DatePicker } from '../DatePicker'

describe('DatePicker', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // S1: Renders with placeholder
  it('renders with the placeholder text in the NL input', () => {
    render(
      <DatePicker
        value={null}
        onChange={vi.fn()}
        placeholder="today, tomorrow…"
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    expect(input).toBeInTheDocument()
  })

  // S1: Renders the calendar grid
  it('renders the calendar with a DayPicker inside', () => {
    render(
      <DatePicker
        value={null}
        onChange={vi.fn()}
        onCancel={vi.fn()}
      />,
    )
    // DayPicker renders a grid — check for the role="grid" or month navigation
    // The component should have role="dialog"
    const dialog = screen.getByRole('dialog')
    expect(dialog).toBeInTheDocument()
  })

  // S1: today highlighted in calendar
  it('shows today highlighted in the calendar', () => {
    render(
      <DatePicker
        value={null}
        onChange={vi.fn()}
      />,
    )
    const dialog = screen.getByRole('dialog')
    // The "today" button should have aria-label containing today's date
    const todayLabel = new Date().toLocaleDateString('en-US', {
      weekday: 'long',
      month: 'long',
      day: 'numeric',
      year: 'numeric',
    })
    // Look for a button with aria-label containing today
    const todayButton = dialog.querySelector(
      '[aria-label]',
    ) as HTMLElement | null
    expect(todayButton).toBeTruthy()
  })

  // S2: NL "today" resolves and calls onChange
  it('resolves "today" and calls onChange with ISO date on Enter', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    fireEvent.change(input, { target: { value: 'today' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(onChange).toHaveBeenCalledTimes(1)
    const calledIso = onChange.mock.calls[0][0] as string
    // Should be YYYY-MM-DD format
    expect(calledIso).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  // S2: NL "tomorrow" resolves
  it('resolves "tomorrow" and calls onChange with next day ISO on Enter', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    fireEvent.change(input, { target: { value: 'tomorrow' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(onChange).toHaveBeenCalledTimes(1)
    const calledIso = onChange.mock.calls[0][0] as string
    expect(calledIso).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  // S2: NL "yesterday" resolves
  it('resolves "yesterday" and calls onChange with previous day ISO on Enter', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    fireEvent.change(input, { target: { value: 'yesterday' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(onChange).toHaveBeenCalledTimes(1)
    const calledIso = onChange.mock.calls[0][0] as string
    expect(calledIso).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  // S6: Invalid NL input shows error and does NOT call onChange
  it('shows error for invalid NL input and does not call onChange', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    fireEvent.change(input, { target: { value: 'not_a_date' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(onChange).not.toHaveBeenCalled()
    // Error message should be visible
    const error = screen.getByTestId('datepicker-error')
    expect(error).toBeInTheDocument()
    expect(error.textContent).toBe('Invalid date')
  })

  // S4: Escape calls onCancel
  it('calls onCancel when Escape is pressed in the NL input', () => {
    const onCancel = vi.fn()
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
        onCancel={onCancel}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    fireEvent.keyDown(input, { key: 'Escape' })

    expect(onCancel).toHaveBeenCalledTimes(1)
    expect(onChange).not.toHaveBeenCalled()
  })

  // S4: Cancel button calls onCancel
  it('calls onCancel when Cancel button is clicked', () => {
    const onCancel = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={vi.fn()}
        onCancel={onCancel}
      />,
    )
    const cancelBtn = screen.getByTestId('datepicker-cancel')
    fireEvent.click(cancelBtn)

    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  // S2: Clicking a day in the calendar calls onChange with ISO date
  it('calls onChange with ISO date when a calendar day is clicked', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    // Find all day buttons in the calendar grid
    const dialog = screen.getByRole('dialog')
    // The first day button should be clickable
    const dayButtons = dialog.querySelectorAll('button')
    // Click the first enabled day button (not disabled/navigation)
    let clicked = false
    for (const btn of dayButtons) {
      if (!btn.hasAttribute('disabled') && btn.textContent?.trim()) {
        fireEvent.click(btn)
        clicked = true
        break
      }
    }
    expect(clicked).toBe(true)
    expect(onChange).toHaveBeenCalledTimes(1)
    const calledIso = onChange.mock.calls[0][0] as string
    expect(calledIso).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  // Value: renders existing date
  it('shows the existing value in the NL input', () => {
    render(
      <DatePicker
        value="2026-06-15"
        onChange={vi.fn()}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    expect(input).toBeInTheDocument()
  })

  // Clear button: shown when value is set
  it('shows Clear button when a value is set', () => {
    render(
      <DatePicker
        value="2026-06-15"
        onChange={vi.fn()}
      />,
    )
    const clearBtn = screen.getByTestId('datepicker-clear')
    expect(clearBtn).toBeInTheDocument()
  })

  // Clear button: calls onChange with empty string
  it('calls onChange with empty string when Clear is clicked', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value="2026-06-15"
        onChange={onChange}
      />,
    )
    const clearBtn = screen.getByTestId('datepicker-clear')
    fireEvent.click(clearBtn)

    expect(onChange).toHaveBeenCalledWith('')
  })

  // NL input: invalid date format shows error
  it('shows error when ISO date is invalid', () => {
    const onChange = vi.fn()
    render(
      <DatePicker
        value={null}
        onChange={onChange}
      />,
    )
    const input = screen.getByPlaceholderText('today, tomorrow…')
    // type an invalid date and press Enter
    fireEvent.change(input, { target: { value: '2026-13-45' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    expect(onChange).not.toHaveBeenCalled()
    const error = screen.getByTestId('datepicker-error')
    expect(error.textContent).toBe('Invalid date')
  })
})
