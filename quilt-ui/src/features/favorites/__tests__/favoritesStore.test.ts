/**
 * Tests for the favorites store — the single source of truth
 * shared by the Sidebar's Favorites section and the PageView's
 * star button (F2 of quilt-fase2-ux-dead-buttons).
 *
 * The store reads/writes `quilt-favorites` in localStorage and
 * fires `quilt:favorites-changed` on every successful mutation
 * so any open view can re-read without polling.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { favoritesStore, FAVORITES_CHANGED_EVENT } from '../favoritesStore'
import { STORAGE_KEYS } from '@features/sidebar/storage-keys'

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('favoritesStore', () => {
  describe('read', () => {
    it('returns an empty list when nothing is stored', () => {
      expect(favoritesStore.read()).toEqual([])
    })

    it('returns the stored list as-is when it is valid', () => {
      localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(['alpha', 'beta']))
      expect(favoritesStore.read()).toEqual(['alpha', 'beta'])
    })

    it('drops non-string entries and returns the survivors', () => {
      localStorage.setItem(
        STORAGE_KEYS.FAVORITES,
        // mixed: a string, a number, an object — only the string survives
        JSON.stringify(['alpha', 42, { name: 'x' }]),
      )
      expect(favoritesStore.read()).toEqual(['alpha'])
    })

    it('returns [] on malformed JSON (does not throw)', () => {
      localStorage.setItem(STORAGE_KEYS.FAVORITES, 'not json{')
      expect(favoritesStore.read()).toEqual([])
    })
  })

  describe('isFavorite', () => {
    it('returns true for stored names and false otherwise', () => {
      localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(['pinned']))
      expect(favoritesStore.isFavorite('pinned')).toBe(true)
      expect(favoritesStore.isFavorite('not-pinned')).toBe(false)
    })
  })

  describe('toggle', () => {
    it('adds a name that is not yet a favorite', () => {
      const next = favoritesStore.toggle('new-page')
      expect(next).toEqual(['new-page'])
      expect(JSON.parse(localStorage.getItem(STORAGE_KEYS.FAVORITES)!)).toEqual(['new-page'])
    })

    it('removes a name that is already a favorite', () => {
      localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(['pinned', 'other']))
      const next = favoritesStore.toggle('pinned')
      expect(next).toEqual(['other'])
    })

    it('fires a quilt:favorites-changed event with the new state', () => {
      const handler = vi.fn()
      window.addEventListener(FAVORITES_CHANGED_EVENT, handler)
      favoritesStore.toggle('page-x')
      expect(handler).toHaveBeenCalledTimes(1)
      // The event detail tells listeners which name flipped and
      // whether it is now a favorite. We read the field from the
      // first arg directly without a generic helper.
      const detail = (handler.mock.calls[0]![0] as CustomEvent<{ name: string; isFavorite: boolean }>).detail
      expect(detail).toEqual({ name: 'page-x', isFavorite: true })
      window.removeEventListener(FAVORITES_CHANGED_EVENT, handler)
    })

    it('dispatches with isFavorite=false when removing a name', () => {
      localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(['pinned']))
      const handler = vi.fn()
      window.addEventListener(FAVORITES_CHANGED_EVENT, handler)
      favoritesStore.toggle('pinned')
      const detail = (handler.mock.calls[0]![0] as CustomEvent<{ name: string; isFavorite: boolean }>).detail
      expect(detail.isFavorite).toBe(false)
      window.removeEventListener(FAVORITES_CHANGED_EVENT, handler)
    })

    it('still fires the event when localStorage is unavailable (defensive)', () => {
      // Simulate a write failure — the in-memory read should still
      // be source-of-truth for this tab.
      const setItemSpy = vi
        .spyOn(Storage.prototype, 'setItem')
        .mockImplementation(() => {
          throw new Error('quota exceeded')
        })
      const handler = vi.fn()
      window.addEventListener(FAVORITES_CHANGED_EVENT, handler)
      const next = favoritesStore.toggle('pinned')
      expect(next).toEqual(['pinned'])
      expect(handler).toHaveBeenCalled()
      setItemSpy.mockRestore()
      window.removeEventListener(FAVORITES_CHANGED_EVENT, handler)
    })
  })
})
