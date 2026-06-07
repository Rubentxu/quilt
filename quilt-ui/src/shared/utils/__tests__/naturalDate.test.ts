/**
 * Tests for naturalDate utilities — convert natural-language date tokens
 * ("today", "tomorrow", "yesterday") to ISO YYYY-MM-DD strings.
 *
 * NL Dates V1 is frontend-only: this is a pure transform used by the
 * property-value input flow. V2 would be a DSL parser with `@today`
 * literals; this V1 lives at the UI boundary so the persisted content
 * is always a real ISO date.
 */
import { describe, it, expect } from 'vitest'
import {
  resolveNaturalDate,
  resolveNaturalDatesInContent,
  isDatePropertyKey,
} from '../naturalDate'

// ── resolveNaturalDate ─────────────────────────────────────────

describe('resolveNaturalDate', () => {
  // Anchor ref date: Friday 2026-06-05 (local).
  // 2026-06-04 → yesterday, 2026-06-05 → today, 2026-06-06 → tomorrow.
  const refDate = new Date(2026, 5, 5)

  it("resolves 'today' to the reference date's YYYY-MM-DD", () => {
    expect(resolveNaturalDate('today', refDate)).toBe('2026-06-05')
  })

  it("resolves 'tomorrow' to the next day", () => {
    expect(resolveNaturalDate('tomorrow', refDate)).toBe('2026-06-06')
  })

  it("resolves 'yesterday' to the previous day", () => {
    expect(resolveNaturalDate('yesterday', refDate)).toBe('2026-06-04')
  })

  it('is case-insensitive', () => {
    expect(resolveNaturalDate('TODAY', refDate)).toBe('2026-06-05')
    expect(resolveNaturalDate('Tomorrow', refDate)).toBe('2026-06-06')
    expect(resolveNaturalDate('YESTERDAY', refDate)).toBe('2026-06-04')
  })

  it('trims surrounding whitespace', () => {
    expect(resolveNaturalDate('  today  ', refDate)).toBe('2026-06-05')
    expect(resolveNaturalDate('\ttoday\n', refDate)).toBe('2026-06-05')
  })

  it('returns null for unknown input', () => {
    expect(resolveNaturalDate('foo', refDate)).toBeNull()
    expect(resolveNaturalDate('next monday', refDate)).toBeNull()
    expect(resolveNaturalDate('in 3 days', refDate)).toBeNull()
  })

  it('returns null for empty string', () => {
    expect(resolveNaturalDate('', refDate)).toBeNull()
    expect(resolveNaturalDate('   ', refDate)).toBeNull()
  })

  it('returns null when the input is already a real date (no work to do)', () => {
    expect(resolveNaturalDate('2026-01-15', refDate)).toBeNull()
    expect(resolveNaturalDate('2026-06-05', refDate)).toBeNull()
  })

  it('pads single-digit month and day with zeros', () => {
    const march3 = new Date(2026, 2, 3)
    expect(resolveNaturalDate('today', march3)).toBe('2026-03-03')
    expect(resolveNaturalDate('yesterday', march3)).toBe('2026-03-02')
    expect(resolveNaturalDate('tomorrow', march3)).toBe('2026-03-04')
  })

  it('handles month and year boundaries', () => {
    const jan1 = new Date(2026, 0, 1)
    expect(resolveNaturalDate('yesterday', jan1)).toBe('2025-12-31')
    expect(resolveNaturalDate('today', jan1)).toBe('2026-01-01')
    expect(resolveNaturalDate('tomorrow', jan1)).toBe('2026-01-02')

    const dec31 = new Date(2026, 11, 31)
    expect(resolveNaturalDate('tomorrow', dec31)).toBe('2027-01-01')
  })

  it('uses the current date when no refDate is supplied', () => {
    // We can't assert an exact value, but we can assert the format
    // (YYYY-MM-DD) and that it matches "today" against `new Date()`.
    const resolved = resolveNaturalDate('today')
    expect(resolved).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  it('does not resolve substrings or compound tokens', () => {
    // We require an exact (post-trim) match, so 'today!' / 'not today'
    // both fall through to null. This keeps the resolver predictable:
    // only a clean, standalone token resolves.
    expect(resolveNaturalDate('today!', refDate)).toBeNull()
    expect(resolveNaturalDate('not today', refDate)).toBeNull()
    expect(resolveNaturalDate('today later', refDate)).toBeNull()
  })
})

// ── isDatePropertyKey ──────────────────────────────────────────

describe('isDatePropertyKey', () => {
  it('recognises the canonical date property keys', () => {
    expect(isDatePropertyKey('deadline')).toBe(true)
    expect(isDatePropertyKey('scheduled')).toBe(true)
    expect(isDatePropertyKey('date')).toBe(true)
  })

  it('is case-insensitive', () => {
    expect(isDatePropertyKey('Deadline')).toBe(true)
    expect(isDatePropertyKey('SCHEDULED')).toBe(true)
    expect(isDatePropertyKey('Date')).toBe(true)
  })

  it('rejects non-date property keys', () => {
    expect(isDatePropertyKey('status')).toBe(false)
    expect(isDatePropertyKey('priority')).toBe(false)
    expect(isDatePropertyKey('tags')).toBe(false)
    expect(isDatePropertyKey('')).toBe(false)
  })
})

// ── resolveNaturalDatesInContent ──────────────────────────────

describe('resolveNaturalDatesInContent', () => {
  const refDate = new Date(2026, 5, 5) // 2026-06-05

  it('resolves a single date property value', () => {
    expect(resolveNaturalDatesInContent('deadline:: today', refDate)).toBe(
      'deadline:: 2026-06-05',
    )
  })

  it('resolves all three natural-date tokens in property values', () => {
    const content =
      'deadline:: tomorrow\nscheduled:: yesterday\ndate:: today'
    expect(resolveNaturalDatesInContent(content, refDate)).toBe(
      'deadline:: 2026-06-06\nscheduled:: 2026-06-04\ndate:: 2026-06-05',
    )
  })

  it('leaves non-date property values untouched', () => {
    expect(
      resolveNaturalDatesInContent('status:: todo priority:: A', refDate),
    ).toBe('status:: todo priority:: A')
  })

  it('leaves real ISO date values untouched (no double-resolution)', () => {
    expect(
      resolveNaturalDatesInContent('deadline:: 2026-01-15', refDate),
    ).toBe('deadline:: 2026-01-15')
  })

  it('leaves bare "today" / "tomorrow" in free text untouched', () => {
    // We only resolve values that are *just* a natural date token in
    // a date property. Free-text mentions must not be rewritten —
    // the user might be writing a sentence.
    expect(
      resolveNaturalDatesInContent('I will finish this today', refDate),
    ).toBe('I will finish this today')
    expect(
      resolveNaturalDatesInContent('see you tomorrow!', refDate),
    ).toBe('see you tomorrow!')
  })

  it('handles case-insensitive property keys and values', () => {
    expect(
      resolveNaturalDatesInContent('Deadline:: TODAY', refDate),
    ).toBe('Deadline:: 2026-06-05')
  })

  it('returns the input unchanged when there is nothing to resolve', () => {
    expect(resolveNaturalDatesInContent('', refDate)).toBe('')
    expect(resolveNaturalDatesInContent('hello world', refDate)).toBe(
      'hello world',
    )
  })

  it('does not resolve values that contain the natural-date token as a substring', () => {
    // `today` is not a standalone value here, so we leave it alone.
    expect(
      resolveNaturalDatesInContent('deadline:: today afternoon', refDate),
    ).toBe('deadline:: today afternoon')
  })

  it('preserves surrounding content and whitespace', () => {
    expect(
      resolveNaturalDatesInContent(
        '  deadline::   today   \n  scheduled::tomorrow  ',
        refDate,
      ),
    ).toBe('  deadline::   2026-06-05   \n  scheduled::2026-06-06  ')
  })
})
