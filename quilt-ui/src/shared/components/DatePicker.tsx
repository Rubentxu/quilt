// ──── DatePicker — quilt-slash-command-functional-behaviour ──────────────
//
// Wraps react-day-picker v9 with a natural-language text input for V1.
// Accepts three input modes:
//   1. Click on a calendar day
//   2. Type NL text ("today", "tomorrow") and press Enter
//   3. Keyboard navigation in the calendar grid
//
// The component is self-contained and designed to be mounted inside
// a Popover anchored to the trigger element.
//
// V1 scope: date only (no time picker), NL set is {today, tomorrow,
// yesterday}. Weekday names ("friday") are V1.5 (spec §11.2).

import { useEffect, useRef, useState } from 'react'
import { DayPicker } from 'react-day-picker'
import 'react-day-picker/style.css'
import { resolveNaturalDate } from '../utils/naturalDate'

export interface DatePickerProps {
  /** Currently selected date as ISO-8601 string (YYYY-MM-DD) or null. */
  value: string | null
  /** Called with ISO-8601 string (YYYY-MM-DD) when the user commits a date. */
  onChange: (iso: string) => void
  /** Called when the user cancels (Escape, click-outside). */
  onCancel?: () => void
  /** Placeholder shown in the NL text input. */
  placeholder?: string
  /** Test id for the root element. */
  testId?: string
}

/**
 * Convert an ISO-8601 date string (YYYY-MM-DD) to a local JavaScript Date.
 * The date string is assumed to be in the user's local timezone (the date
 * portion only — we treat it as local midnight).
 */
function isoToDate(iso: string): Date {
  const [y, m, d] = iso.split('-').map(Number)
  return new Date(y!, (m! - 1), d!)
}

/**
 * Convert a local Date to ISO-8601 date string (YYYY-MM-DD).
 */
function dateToIso(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

export function DatePicker({
  value,
  onChange,
  onCancel,
  placeholder = 'today, tomorrow…',
  testId,
}: DatePickerProps) {
  // Internal text input state (separate from calendar selection)
  const [inputValue, setInputValue] = useState('')
  const [inputError, setInputError] = useState<string | null>(null)
  // Which month the calendar is showing (controlled for keyboard nav)
  const [month, setMonth] = useState(() =>
    value ? isoToDate(value) : new Date(),
  )
  const inputRef = useRef<HTMLInputElement>(null)
  const rootRef = useRef<HTMLDivElement>(null)

  // Focus the text input on mount
  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  /** Parse the NL input and commit if valid, else show error. */
  function commitNlInput() {
    const trimmed = inputValue.trim()
    if (!trimmed) {
      setInputError(null)
      return
    }
    const resolved = resolveNaturalDate(trimmed)
    if (resolved === null) {
      setInputError('Invalid date')
      return
    }
    setInputError(null)
    setInputValue('')
    onChange(resolved)
  }

  /** Handle day selection from the calendar. */
  function handleDaySelect(date: Date | undefined) {
    if (!date) return
    const iso = dateToIso(date)
    onChange(iso)
  }

  /** Convert the current value prop to a Date for the calendar. */
  const selectedDate = value ? isoToDate(value) : undefined

  return (
    <div
      ref={rootRef}
      data-testid={testId}
      role="dialog"
      aria-label="Select date"
      style={{
        background: 'var(--color-surface, #fff)',
        border: '1px solid var(--color-border, #e5e7eb)',
        borderRadius: '8px',
        padding: '12px',
        width: '280px',
        boxShadow: '0 4px 16px rgba(0,0,0,0.12)',
        fontFamily: 'inherit',
      }}
    >
      {/* NL text input */}
      <input
        ref={inputRef}
        type="text"
        data-testid={testId ? `${testId}-nl-input` : 'datepicker-nl-input'}
        value={inputValue}
        onChange={e => {
          setInputValue(e.target.value)
          setInputError(null)
        }}
        onKeyDown={e => {
          if (e.key === 'Enter') {
            e.preventDefault()
            commitNlInput()
          }
          if (e.key === 'Escape') {
            e.preventDefault()
            onCancel?.()
          }
          // ArrowDown → focus the calendar grid (move focus to the first day button)
          if (e.key === 'ArrowDown') {
            e.preventDefault()
            const firstDay = rootRef.current?.querySelector<HTMLButtonElement>(
              '[aria-label*="Monday"], [data-day]',
            )
            firstDay?.focus()
          }
        }}
        placeholder={placeholder}
        aria-label="Type a date or natural language (today, tomorrow)"
        style={{
          width: '100%',
          padding: '6px 10px',
          borderRadius: '6px',
          border: inputError ? '1.5px solid #ef4444' : '1px solid var(--color-border, #e5e7eb)',
          fontSize: '13px',
          outline: 'none',
          marginBottom: '4px',
          boxSizing: 'border-box',
          background: 'var(--color-surface, #fff)',
          color: 'var(--color-text-primary, #111)',
        }}
      />
      {/* Error message */}
      {inputError && (
        <div
          data-testid={testId ? `${testId}-error` : 'datepicker-error'}
          style={{ color: '#ef4444', fontSize: '11px', marginBottom: '6px' }}
        >
          {inputError}
        </div>
      )}

      {/* Calendar */}
      <div
        style={{ marginTop: '8px' }}
        onKeyDown={e => {
          if (e.key === 'Escape') {
            e.stopPropagation()
            onCancel?.()
          }
        }}
      >
        <DayPicker
          mode="single"
          selected={selectedDate}
          onSelect={handleDaySelect}
          month={month}
          onMonthChange={setMonth}
          style={{
            fontSize: '13px',
          }}
        />
      </div>

      {/* Clear + Cancel buttons */}
      <div
        style={{
          display: 'flex',
          gap: '8px',
          marginTop: '10px',
          justifyContent: 'flex-end',
        }}
      >
        {value && (
          <button
            type="button"
            data-testid={testId ? `${testId}-clear` : 'datepicker-clear'}
            onClick={() => onChange('')} // empty string → caller treats as clear
            style={{
              padding: '4px 12px',
              borderRadius: '6px',
              border: '1px solid var(--color-border, #e5e7eb)',
              background: 'transparent',
              color: 'var(--color-text-secondary, #6b7280)',
              fontSize: '12px',
              cursor: 'pointer',
            }}
          >
            Clear
          </button>
        )}
        <button
          type="button"
          data-testid={testId ? `${testId}-cancel` : 'datepicker-cancel'}
          onClick={() => onCancel?.()}
          style={{
            padding: '4px 12px',
            borderRadius: '6px',
            border: '1px solid var(--color-border, #e5e7eb)',
            background: 'transparent',
            color: 'var(--color-text-secondary, #6b7280)',
            fontSize: '12px',
            cursor: 'pointer',
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  )
}
