// ──── Saved / Recent searches (ROADMAP #22) ────────────────────────
//
// Two flat, frontend-only histories persisted in localStorage:
//
//   - `recent-searches` — auto-recorded every time the user closes the
//     search modal with a non-empty input. Capped at 10 entries with
//     move-to-top dedupe. Powers the "Recent searches" panel that
//     appears in the search modal when the input is empty.
//
//   - `saved-searches` — user-named, opt-in, unbounded. The user
//     explicitly taps "Save search" from the search modal and gives
//     the entry a name. Saved searches are visible alongside the
//     recent list and survive across sessions (until the user
//     deletes them).
//
// This module is **pure logic** (load / save / record / add / delete /
// validate). The React UI that wires it into the SearchModal lives
// in `SearchModal.tsx`. Tests for the pure part live in
// `__tests__/savedSearches.test.ts`; integration tests live in
// `__tests__/SearchModal.savedSearches.test.tsx`.
//
// Dependency direction:
//   - No imports from React, no imports from `@core/api-client`. The
//     persistence layer is a pure functional core that the React
//     layer wraps.

/** localStorage key for the recent (auto-saved) search history. */
export const RECENT_SEARCHES_KEY = 'recent-searches'

/** localStorage key for the user-named, opt-in saved-searches. */
export const SAVED_SEARCHES_KEY = 'saved-searches'

/** Hard cap on the recent-searches history. Oldest entries are evicted. */
export const MAX_RECENT_SEARCHES = 10

/** Recognised SavedView block roles (ROADMAP #20). The `viewType`
 *  is the type of block the search should render as when the user
 *  re-runs the saved search. */
export const SAVED_SEARCH_VIEW_TYPES = [
  'kanban',
  'table',
  'list',
  'graph',
  'cards',
  'calendar',
  'timeline',
] as const

export type SavedSearchViewType = (typeof SAVED_SEARCH_VIEW_TYPES)[number]

/**
 * One entry in the auto-saved recent-searches history.
 *
 * - `query` is the raw user query string (whitespace-trimmed).
 * - `timestamp` is the epoch-ms at which it was last run. Repeated
 *   runs of the same query refresh the timestamp AND move the
 *   entry to the front.
 * - `resultCount` is the total number of hits the last run
 *   produced (sum of pages + blocks). We persist it so the UI can
 *   render a small "12 results" hint next to each recent entry
 *   without re-running the search.
 */
export interface RecentSearch {
  query: string
  timestamp: number
  resultCount: number
}

/**
 * One entry in the user-saved search list.
 *
 * - `id` is a stable, locally-generated UUID-shaped string used as the
 *   React `key` and as the argument to `deleteSavedSearch`. The UI
 *   generates it with `crypto.randomUUID()` when available, falling
 *   back to a small Math.random-based shape.
 * - `name` is the human label the user typed at save time.
 * - `query` is the exact DSL the search modal should re-populate
 *   when the user clicks the saved entry.
 * - `viewType` is optional; when present, downstream views (kanban,
 *   table, etc.) should render results in that shape (Q025).
 * - `createdAt` is the epoch-ms at which the entry was first saved.
 */
export interface SavedSearch {
  id: string
  name: string
  query: string
  viewType?: SavedSearchViewType
  createdAt: number
}

// ──── Type guards (defensive parsing) ──────────────────────────────

/** A read-time type guard for {@link RecentSearch}. */
export function isValidRecentSearch(value: unknown): value is RecentSearch {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  return (
    typeof v.query === 'string' &&
    typeof v.timestamp === 'number' &&
    typeof v.resultCount === 'number'
  )
}

/** A read-time type guard for {@link SavedSearch}. */
export function isValidSavedSearch(value: unknown): value is SavedSearch {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  if (typeof v.id !== 'string' || v.id.length === 0) return false
  if (typeof v.name !== 'string' || v.name.length === 0) return false
  if (typeof v.query !== 'string' || v.query.length === 0) return false
  if (typeof v.createdAt !== 'number') return false
  if (v.viewType !== undefined) {
    if (typeof v.viewType !== 'string') return false
    if (!(SAVED_SEARCH_VIEW_TYPES as readonly string[]).includes(v.viewType)) {
      return false
    }
  }
  return true
}

// ──── localStorage shim guards ─────────────────────────────────────
//
// Both `load*` and `save*` tolerate a missing `localStorage` (private
// mode, SSR, tests without the shim). A failure to read silently
// returns `[]`; a failure to write is a silent no-op. The user's
// history is best-effort, not load-bearing.

function readJson(key: string): unknown {
  if (typeof localStorage === 'undefined') return null
  const raw = localStorage.getItem(key)
  if (raw === null) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function writeJson(key: string, value: unknown): void {
  if (typeof localStorage === 'undefined') return
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Quota exceeded or storage disabled — silently drop the write.
    // Better to lose history than to crash the modal.
  }
}

// ──── Recent searches ─────────────────────────────────────────────

/**
 * Read the persisted recent-searches history. Tolerant of:
 *   - missing key            → []
 *   - malformed JSON         → []
 *   - non-array value        → []
 *   - entries with wrong shape (filtered out; the survivors come back)
 */
export function loadRecentSearches(): RecentSearch[] {
  const parsed = readJson(RECENT_SEARCHES_KEY)
  if (!Array.isArray(parsed)) return []
  return parsed.filter(isValidRecentSearch)
}

/** Overwrite the persisted recent-searches history. */
export function saveRecentSearches(history: RecentSearch[]): void {
  writeJson(RECENT_SEARCHES_KEY, history)
}

/**
 * Record one user-run search into the recent history.
 *
 *   - Empty / whitespace-only queries are NOT recorded (we don't
 *     want a cluttery " " entry when the user opens the modal and
 *     closes it without typing).
 *   - The query is stored **trimmed** (leading / trailing whitespace
 *     stripped) so dedup keys are stable across re-runs.
 *   - Repeated queries are deduped: the existing entry is moved to
 *     the front with the new timestamp and resultCount. The rest of
 *     the list keeps its order (stable for the rest of the history).
 *   - The list is capped at {@link MAX_RECENT_SEARCHES}. When the
 *     cap is hit, the oldest entry is evicted (FIFO from the tail).
 *
 * @param entry     The query and result count to record. The
 *                  timestamp is supplied separately so the caller can
 *                  inject a deterministic clock in tests.
 * @param now       Epoch-ms "now". Pass `Date.now()` in production.
 * @returns         The updated history (also persisted to
 *                  localStorage as a side effect).
 */
export function recordRecentSearch(
  entry: { query: string; resultCount: number },
  now: number,
): RecentSearch[] {
  const query = entry.query.trim()
  if (!query) return loadRecentSearches()

  const next: RecentSearch = {
    query,
    timestamp: now,
    resultCount: entry.resultCount,
  }

  const current = loadRecentSearches().filter(r => r.query !== query)
  const updated = [next, ...current].slice(0, MAX_RECENT_SEARCHES)
  saveRecentSearches(updated)
  return updated
}

// ──── Saved searches ──────────────────────────────────────────────

/**
 * Read the persisted saved-searches list. Same tolerance rules as
 * {@link loadRecentSearches}.
 */
export function loadSavedSearches(): SavedSearch[] {
  const parsed = readJson(SAVED_SEARCHES_KEY)
  if (!Array.isArray(parsed)) return []
  return parsed.filter(isValidSavedSearch)
}

/** Overwrite the persisted saved-searches list. */
export function saveSavedSearches(list: SavedSearch[]): void {
  writeJson(SAVED_SEARCHES_KEY, list)
}

/**
 * Append a new saved search to the persisted list.
 *
 *   - `name` must be non-empty (after trim).
 *   - `query` must be non-empty.
 *   - The `id` is supplied by the caller (a UUID-shaped string).
 *     We use an injected id+timestamp so tests can be deterministic;
 *     the React layer passes `crypto.randomUUID()` and `Date.now()`.
 *
 * @throws when the name or query is empty.
 */
export function addSavedSearch(
  entry: { name: string; query: string; viewType?: SavedSearchViewType },
  makeId: () => string,
  now: () => number,
): SavedSearch {
  const name = entry.name.trim()
  const query = entry.query.trim()
  if (!name) throw new Error('Saved search name is required')
  if (!query) throw new Error('Saved search query is required')

  const created: SavedSearch = {
    id: makeId(),
    name,
    query,
    viewType: entry.viewType,
    createdAt: now(),
  }
  const list = loadSavedSearches()
  saveSavedSearches([...list, created])
  return created
}

/** Remove a saved search by id. No-op if the id is unknown. */
export function deleteSavedSearch(id: string): void {
  const list = loadSavedSearches()
  const next = list.filter(s => s.id !== id)
  if (next.length === list.length) return // nothing changed
  saveSavedSearches(next)
}
