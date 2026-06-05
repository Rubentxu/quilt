// Central registry of localStorage keys owned by the sidebar feature.
//
// `quilt-*` is the project's informal namespace for keys read by the
// Quilt frontend. Centralising the names here gives us a single audit
// point for what the sidebar persists, prevents typo-induced key
// collisions, and lets the rest of the sidebar import a typed const
// instead of a string literal.
//
// New keys MUST be added here rather than declared inline in feature
// code — the auto-grill decision (see design.md) recommended this
// shared module precisely to avoid the historical pattern of
// `localStorage.getItem('quilt-<x>')` scattered through components.

export const STORAGE_KEYS = {
  /** Page names the user has starred (DESIGN.md §4.1). */
  FAVORITES: 'quilt-favorites',
  /** Most recently visited pages, capped at 5, newest first. */
  RECENTS: 'quilt-recents',
  /**
   * "1" when the user has dismissed the first-run welcome tour. The
   * tour explains the four key Quilt primitives (Plantillas, Recents,
   * Slash command, Properties) and only re-appears if the user
   * clears this flag. Quilt fase 2 empty-states fix.
   */
  WELCOME_SEEN: 'quilt-welcome-seen',
} as const

export type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS]
