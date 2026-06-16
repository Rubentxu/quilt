/**
 * Tests for queryHistory pure module.
 */

import { describe, it, expect, beforeEach } from 'vitest'
import {
  QUERY_HISTORY_KEY,
  MAX_QUERY_HISTORY,
  loadQueryHistory,
  saveQueryHistory,
  recordQueryExecution,
  clearQueryHistory,
  isValidQueryHistoryEntry,
  type QueryHistoryEntry,
} from '../queryHistory'

beforeEach(() => {
  localStorage.clear()
})

const now = () => 1_700_000_000_000

describe('storage keys', () => {
  it('uses the spec key query-history', () => {
    expect(QUERY_HISTORY_KEY).toBe('query-history')
  })

  it('caps history at MAX_QUERY_HISTORY', () => {
    expect(MAX_QUERY_HISTORY).toBe(10)
  })
})

describe('isValidQueryHistoryEntry', () => {
  it('returns true for a valid entry', () => {
    const entry: QueryHistoryEntry = {
      query: '(task todo)',
      timestamp: 1_700_000_000_000,
      resultCount: 5,
    }
    expect(isValidQueryHistoryEntry(entry)).toBe(true)
  })

  it('returns false for missing query', () => {
    const entry = { timestamp: 1, resultCount: 1 } as unknown as QueryHistoryEntry
    expect(isValidQueryHistoryEntry(entry)).toBe(false)
  })

  it('returns false for missing timestamp', () => {
    const entry = { query: 'x', resultCount: 1 } as unknown as QueryHistoryEntry
    expect(isValidQueryHistoryEntry(entry)).toBe(false)
  })

  it('returns false for non-object', () => {
    expect(isValidQueryHistoryEntry(null)).toBe(false)
    expect(isValidQueryHistoryEntry('string')).toBe(false)
    expect(isValidQueryHistoryEntry(42)).toBe(false)
  })
})

describe('loadQueryHistory', () => {
  it('returns empty array when key is missing', () => {
    expect(loadQueryHistory()).toEqual([])
  })

  it('returns empty array when stored value is not an array', () => {
    localStorage.setItem(QUERY_HISTORY_KEY, 'not an array')
    expect(loadQueryHistory()).toEqual([])
  })

  it('filters out invalid entries', () => {
    localStorage.setItem(
      QUERY_HISTORY_KEY,
      JSON.stringify([
        { query: '(task todo)', timestamp: 1, resultCount: 5 },
        { timestamp: 1, resultCount: 1 }, // invalid — missing query
        { query: '(page "x")', timestamp: 2, resultCount: 2 }, // valid
      ]),
    )
    const history = loadQueryHistory()
    expect(history).toHaveLength(2)
    expect(history[0].query).toBe('(task todo)')
    expect(history[1].query).toBe('(page "x")')
  })
})

describe('recordQueryExecution', () => {
  it('records a new query at the front', () => {
    const history = recordQueryExecution({ query: '(task todo)', resultCount: 3 }, now())
    expect(history).toHaveLength(1)
    expect(history[0].query).toBe('(task todo)')
    expect(history[0].resultCount).toBe(3)
  })

  it('moves existing query to front on repeat', () => {
    recordQueryExecution({ query: '(task todo)', resultCount: 1 }, now())
    const history = recordQueryExecution({ query: '(task todo)', resultCount: 5 }, now() + 1000)
    expect(history).toHaveLength(1)
    expect(history[0].resultCount).toBe(5)
  })

  it('caps at MAX_QUERY_HISTORY', () => {
    // Record 12 queries
    for (let i = 0; i < 12; i++) {
      recordQueryExecution({ query: `(page "p${i}")`, resultCount: i }, now() + i * 1000)
    }
    const history = loadQueryHistory()
    expect(history).toHaveLength(MAX_QUERY_HISTORY)
    // Most recent should be p11
    expect(history[0].query).toBe('(page "p11")')
  })

  it('does not record empty query', () => {
    const history = recordQueryExecution({ query: '   ', resultCount: 0 }, now())
    expect(history).toHaveLength(0)
  })
})

describe('clearQueryHistory', () => {
  it('clears all history', () => {
    recordQueryExecution({ query: '(task todo)', resultCount: 1 }, now())
    clearQueryHistory()
    expect(loadQueryHistory()).toHaveLength(0)
  })
})
