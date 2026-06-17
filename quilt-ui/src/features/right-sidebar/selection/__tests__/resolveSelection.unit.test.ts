// ─── resolveSelection.unit.test ─────────────────────────────────────────────

import { describe, it, expect } from 'vitest'
import { resolveSelection } from '../resolveSelection'

describe('resolveSelection', () => {
  describe('block-level selection', () => {
    it('returns BlockSelection when blockId is present', () => {
      const result = resolveSelection({
        pathname: '/page/test-page',
        blockId: 'block-123',
        nextRouteKey: 'page/test-page',
      })
      expect(result).toEqual({
        type: 'block',
        blockId: 'block-123',
        pageName: 'test-page',
      })
    })

    it('extracts pageName from /journal/YYYY-MM-DD', () => {
      const result = resolveSelection({
        pathname: '/journal/2026-06-17',
        blockId: 'block-456',
        nextRouteKey: 'journal/2026-06-17',
      })
      expect(result).toEqual({
        type: 'block',
        blockId: 'block-456',
        pageName: '2026-06-17',
      })
    })

    it('ignores blockId when prevRouteKey !== nextRouteKey (route-key guard)', () => {
      const result = resolveSelection({
        pathname: '/page/test-page',
        blockId: 'block-123',
        prevRouteKey: 'page/old-page',
        nextRouteKey: 'page/test-page',
      })
      // Block selection should be cleared on navigation
      expect(result?.type).toBe('page')
      expect(result).toEqual({
        type: 'page',
        pageName: 'test-page',
        isJournal: false,
      })
    })

    it('ignores empty blockId', () => {
      const result = resolveSelection({
        pathname: '/page/test-page',
        blockId: '',
        nextRouteKey: 'page/test-page',
      })
      expect(result?.type).toBe('page')
    })

    it('ignores null blockId', () => {
      const result = resolveSelection({
        pathname: '/page/test-page',
        blockId: null,
        nextRouteKey: 'page/test-page',
      })
      expect(result?.type).toBe('page')
    })
  })

  describe('page-level selection', () => {
    it('returns PageSelection for /page/<name>', () => {
      const result = resolveSelection({
        pathname: '/page/my-page',
        nextRouteKey: 'page/my-page',
      })
      expect(result).toEqual({
        type: 'page',
        pageName: 'my-page',
        isJournal: false,
      })
    })

    it('returns PageSelection with isJournal=true for /journal/YYYY-MM-DD', () => {
      const result = resolveSelection({
        pathname: '/journal/2026-06-17',
        nextRouteKey: 'journal/2026-06-17',
      })
      expect(result).toEqual({
        type: 'page',
        pageName: '2026-06-17',
        isJournal: true,
      })
    })

    it('URL-decodes page names', () => {
      const result = resolveSelection({
        pathname: '/page/hello%20world',
        nextRouteKey: 'page/hello%20world',
      })
      expect(result).toEqual({
        type: 'page',
        pageName: 'hello world',
        isJournal: false,
      })
    })

    it('handles malformed URI gracefully', () => {
      const result = resolveSelection({
        pathname: '/page/hello%ZZ',
        nextRouteKey: 'page/hello%ZZ',
      })
      // Should not throw, returns page selection with raw segment
      expect(result?.type).toBe('page')
    })
  })

  describe('graph-level selection (fallback)', () => {
    it('returns null for /graph', () => {
      const result = resolveSelection({
        pathname: '/graph',
        nextRouteKey: 'graph',
      })
      expect(result).toBe(null)
    })

    it('returns null for /pages', () => {
      const result = resolveSelection({
        pathname: '/pages',
        nextRouteKey: 'pages',
      })
      expect(result).toBe(null)
    })

    it('returns null for /settings', () => {
      const result = resolveSelection({
        pathname: '/settings',
        nextRouteKey: 'settings',
      })
      expect(result).toBe(null)
    })

    it('returns null for / (root)', () => {
      const result = resolveSelection({
        pathname: '/',
        nextRouteKey: '/',
      })
      expect(result).toBe(null)
    })
  })
})
