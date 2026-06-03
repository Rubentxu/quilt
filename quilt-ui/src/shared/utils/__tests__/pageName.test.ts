import { describe, it, expect } from 'vitest'
import { normalizePageName } from '../pageName'

// The server's Page::normalize_name (crates/quilt-domain/src/entities/page.rs)
// is the source of truth for page-name canonicalisation. This client-side
// helper mirrors it so the frontend can lookup, create, and navigate
// pages without a case mismatch. These tests pin the contract from
// the user's perspective.

describe('normalizePageName', () => {
  it('lowercases a mixed-case name', () => {
    expect(normalizePageName('MyNotes')).toBe('mynotes')
  })

  it('trims surrounding whitespace', () => {
    expect(normalizePageName('  spaced  ')).toBe('spaced')
    expect(normalizePageName('\tmixed\n')).toBe('mixed')
  })

  it('lowercases AND trims at once', () => {
    expect(normalizePageName('  My Notes  ')).toBe('my notes')
  })

  it('preserves spaces inside the name (the server does not collapse them)', () => {
    // The server treats 'page with spaces' as a single valid name; the
    // URL-encoding happens at the routing layer, not here.
    expect(normalizePageName('Page With Spaces')).toBe('page with spaces')
  })

  it('preserves digits, dashes, and underscores', () => {
    expect(normalizePageName('TODO-2026-01-15')).toBe('todo-2026-01-15')
    expect(normalizePageName('draft_v3')).toBe('draft_v3')
  })

  it('returns an empty string for an all-whitespace input', () => {
    expect(normalizePageName('   ')).toBe('')
    expect(normalizePageName('\n\t  ')).toBe('')
  })

  it('is idempotent — already-canonical input is a no-op', () => {
    const input = 'already-canonical'
    expect(normalizePageName(normalizePageName(input))).toBe(input)
  })

  it('lowercases uppercase Unicode (best-effort, not exhaustive)', () => {
    // JavaScript's toLowerCase is locale-insensitive for ASCII and
    // maps most common case pairs. We don't promise Turkish-dotted-i
    // handling — the server also uses str::to_lowercase which is
    // Unicode-aware but does not cover every edge case.
    expect(normalizePageName('CAFÉ')).toBe('café')
  })
})
