import { describe, it, expect } from 'vitest'
import { deriveCurrentPageName } from '../AppShell'

describe('AppShell — deriveCurrentPageName (G6: backlinks on every page)', () => {
  it('returns the decoded page name for /page/<name> routes', () => {
    expect(deriveCurrentPageName('/page/Foo')).toBe('Foo')
    expect(deriveCurrentPageName('/page/Foo%20Bar')).toBe('Foo Bar')
  })

  it('returns the date for /journal/<YYYY-MM-DD> routes (G6 fix)', () => {
    // The pre-fix code only handled /page/ paths, so journal pages
    // were shown without any page name — the panel rendered but had
    // nothing to query. This test pins down the fix.
    expect(deriveCurrentPageName('/journal/2026-06-03')).toBe('2026-06-03')
  })

  it('returns null for the home route', () => {
    expect(deriveCurrentPageName('/')).toBeNull()
  })

  it('returns null for non-page routes', () => {
    expect(deriveCurrentPageName('/settings')).toBeNull()
    expect(deriveCurrentPageName('/pages')).toBeNull()
    expect(deriveCurrentPageName('/graph')).toBeNull()
  })

  it('returns null for empty /page/ segments', () => {
    // Defensive: trailing slash or accidental empty segment
    expect(deriveCurrentPageName('/page/')).toBeNull()
    expect(deriveCurrentPageName('/page')).toBeNull()
  })

  it('returns null for empty /journal/ segments', () => {
    expect(deriveCurrentPageName('/journal/')).toBeNull()
    expect(deriveCurrentPageName('/journal')).toBeNull()
  })

  it('handles nested page names (e.g. namespaced pages)', () => {
    expect(deriveCurrentPageName('/page/Projects%2F2026')).toBe('Projects/2026')
  })
})
