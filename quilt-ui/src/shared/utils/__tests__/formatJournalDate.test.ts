import { describe, it, expect } from 'vitest'
import { formatJournalDate } from '../formatJournalDate'

describe('formatJournalDate', () => {
  // Monday, June 15, 2026 (local time)
  const date = new Date(2026, 5, 15)

  it('formats ISO default (YYYY-MM-DD)', () => {
    expect(formatJournalDate(date, '%Y-%m-%d')).toBe('2026-06-15')
  })

  it('formats European style (DD/MM/YYYY)', () => {
    expect(formatJournalDate(date, '%d/%m/%Y')).toBe('15/06/2026')
  })

  it('formats US style (MM/DD/YYYY)', () => {
    expect(formatJournalDate(date, '%m/%d/%Y')).toBe('06/15/2026')
  })

  it('formats long with weekday', () => {
    expect(formatJournalDate(date, '%A, %B %d, %Y')).toBe('Monday, June 15, 2026')
  })

  it('formats abbreviated weekday + month + day + year', () => {
    expect(formatJournalDate(date, '%A %b %d %Y')).toBe('Monday Jun 15 2026')
  })

  it('pads single-digit month and day with zeros', () => {
    const jan3 = new Date(2026, 0, 3)
    expect(formatJournalDate(jan3, '%Y-%m-%d')).toBe('2026-01-03')
  })

  it('returns the format string unchanged when no placeholders are present', () => {
    expect(formatJournalDate(date, 'Hello world')).toBe('Hello world')
  })

  it('leaves unknown placeholders intact (so user mistakes surface visibly)', () => {
    expect(formatJournalDate(date, '%Y-%X')).toBe('2026-%X')
  })

  it('replaces repeated placeholders', () => {
    // %Y should appear 3 times.
    expect(formatJournalDate(date, '%Y-%Y-%Y')).toBe('2026-2026-2026')
  })
})
