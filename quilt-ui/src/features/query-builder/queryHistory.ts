// ─── Query History ─────────────────────────────────────────────────
//
// Persisted in localStorage, FIFO-capped at 10 entries.
//
// Unlike SearchModal's recent-searches (which track FTS queries),
// this module tracks DSL query strings executed via the QueryBuilder.
//
// Dependency direction:
//   - No imports from React, no imports from `@core/api-client`.
//     Pure functional core wrapped by the React QueryBuilder layer.

/** localStorage key for the DSL query execution history. */
export const QUERY_HISTORY_KEY = 'query-history'

/** Hard cap on the query history. Oldest entries are evicted. */
export const MAX_QUERY_HISTORY = 10

/**
 * One entry in the DSL query execution history.
 *
 * - `query` is the raw DSL string the user executed.
 * - `timestamp` is the epoch-ms at which it was last run.
 * - `resultCount` is the number of results returned.
 */
export interface QueryHistoryEntry {
  query: string
  timestamp: number
  resultCount: number
}

// ──── Type guards ─────────────────────────────────────────────────

/** A read-time type guard for {@link QueryHistoryEntry}. */
export function isValidQueryHistoryEntry(value: unknown): value is QueryHistoryEntry {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  return (
    typeof v.query === 'string' &&
    typeof v.timestamp === 'number' &&
    typeof v.resultCount === 'number'
  )
}

// ──── localStorage shim ──────────────────────────────────────────

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
    // Silent no-op
  }
}

// ──── Public API ────────────────────────────────────────────────

/**
 * Read the persisted query history.
 * Tolerates: missing key, malformed JSON, non-array → []
 */
export function loadQueryHistory(): QueryHistoryEntry[] {
  const parsed = readJson(QUERY_HISTORY_KEY)
  if (!Array.isArray(parsed)) return []
  return parsed.filter(isValidQueryHistoryEntry)
}

/** Overwrite the persisted query history. */
export function saveQueryHistory(history: QueryHistoryEntry[]): void {
  writeJson(QUERY_HISTORY_KEY, history)
}

/**
 * Record one DSL query execution into the history.
 *
 * - Empty / whitespace queries are NOT recorded.
 * - Repeated queries are deduped: existing entry moves to front with
 *   new timestamp and resultCount.
 * - List is capped at MAX_QUERY_HISTORY (FIFO eviction).
 *
 * @param entry   The query and result count to record.
 * @param now     Epoch-ms "now". Pass `Date.now()` in production.
 * @returns       Updated history (also persisted as side effect).
 */
export function recordQueryExecution(
  entry: { query: string; resultCount: number },
  now: number,
): QueryHistoryEntry[] {
  const query = entry.query.trim()
  if (!query) return loadQueryHistory()

  const next: QueryHistoryEntry = {
    query,
    timestamp: now,
    resultCount: entry.resultCount,
  }

  const current = loadQueryHistory().filter(r => r.query !== query)
  const updated = [next, ...current].slice(0, MAX_QUERY_HISTORY)
  saveQueryHistory(updated)
  return updated
}

/** Clear all query history. */
export function clearQueryHistory(): void {
  saveQueryHistory([])
}
