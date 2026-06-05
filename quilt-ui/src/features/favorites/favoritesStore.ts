// ─── favorites — single source of truth for the favorites list ───
//
// The Sidebar (feature/sidebar/Sidebar.tsx) is the canonical UI for
// viewing and removing favorites, but the page header (PageView.tsx)
// also needs to *add* a favorite from the current page without
// forcing a full reload. Both call sites read/write the same
// `STORAGE_KEYS.FAVORITES` localStorage key, so we centralise:
//
//   - readFavorites():   returns the current list (defensive parse)
//   - isFavorite(name):  single-name check
//   - toggleFavorite(name): flips and persists, returns next list
//   - FAVORITES_CHANGED_EVENT: CustomEvent name dispatched on every
//     mutation, so any open Sidebar / PageView can re-read the
//     list without polling.
//
// Keeping the event name as a const (rather than two free strings)
// means a typo in one component can't silently desync them.

import { STORAGE_KEYS } from '@features/sidebar/storage-keys'

/** Dispatched on `window` after every successful `toggleFavorite`. */
export const FAVORITES_CHANGED_EVENT = 'quilt:favorites-changed'

function readFavorites(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.FAVORITES)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed.filter((v): v is string => typeof v === 'string')
  } catch {
    return []
  }
}

function isFavorite(name: string): boolean {
  return readFavorites().includes(name)
}

/**
 * Flip the favorite state for `name` and persist the result. The
 * returned array is the new list (caller can use it for an
 * immediate UI update without re-reading storage). After persisting,
 * a `quilt:favorites-changed` event is fired on `window` so any
 * other view that shows the favorites (e.g. the sidebar) can
 * re-read.
 */
function toggleFavorite(name: string): string[] {
  const current = readFavorites()
  const next = current.includes(name)
    ? current.filter((n) => n !== name)
    : [...current, name]
  try {
    localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(next))
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent(FAVORITES_CHANGED_EVENT, { detail: { name, isFavorite: !current.includes(name) } }))
    }
  } catch {
    // localStorage may be unavailable (private mode, quota). The
    // in-memory list is the source of truth for the lifetime of
    // this tab — the event still fires so listeners can re-read.
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent(FAVORITES_CHANGED_EVENT, { detail: { name, isFavorite: !current.includes(name) } }))
    }
  }
  return next
}

export const favoritesStore = {
  read: readFavorites,
  isFavorite,
  toggle: toggleFavorite,
} as const
