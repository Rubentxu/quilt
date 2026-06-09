// Tests for the saved/recent searches persistence module.
//
// This file is the lowest layer of the ROADMAP #22 test pyramid:
// pure functions (load / save / record / delete) plus their
// invariants. The SearchModal UI tests in
// `SearchModal.savedSearches.test.tsx` exercise the *integration* of
// this module with the modal, but the eviction / validation logic is
// owned here so the rules are testable without React.
//
// The storage keys (`recent-searches`, `saved-searches`) and the
// shape of the data are fixed by the ROADMAP #22 spec — keep them
// in sync with the matching keys in
// `quilt-ui/src/features/sidebar/storage-keys.ts`.

import { describe, it, expect, beforeEach } from 'vitest'
import {
  RECENT_SEARCHES_KEY,
  SAVED_SEARCHES_KEY,
  MAX_RECENT_SEARCHES,
  loadRecentSearches,
  saveRecentSearches,
  recordRecentSearch,
  loadSavedSearches,
  saveSavedSearches,
  addSavedSearch,
  deleteSavedSearch,
  isValidRecentSearch,
  isValidSavedSearch,
  type RecentSearch,
  type SavedSearch,
} from '../savedSearches'

beforeEach(() => {
  localStorage.clear()
})

const now = () => 1_700_000_000_000

describe('storage keys', () => {
  it('uses the spec keys (recent-searches, saved-searches)', () => {
    expect(RECENT_SEARCHES_KEY).toBe('recent-searches')
    expect(SAVED_SEARCHES_KEY).toBe('saved-searches')
  })

  it('caps the recent-search history at 10 entries', () => {
    expect(MAX_RECENT_SEARCHES).toBe(10)
  })
})

describe('loadRecentSearches', () => {
  it('returns an empty array when localStorage has no key', () => {
    expect(loadRecentSearches()).toEqual([])
  })

  it('returns the parsed array on a happy-path read', () => {
    const items: RecentSearch[] = [
      { query: 'foo', timestamp: 1, resultCount: 3 },
    ]
    localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(items))
    expect(loadRecentSearches()).toEqual(items)
  })

  it('returns an empty array on malformed JSON', () => {
    localStorage.setItem(RECENT_SEARCHES_KEY, 'not json{')
    expect(loadRecentSearches()).toEqual([])
  })

  it('returns an empty array when the stored value is not an array', () => {
    localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify({ foo: 'bar' }))
    expect(loadRecentSearches()).toEqual([])
  })

  it('filters out malformed entries (defensive parse)', () => {
    const dirty = [
      { query: 'foo', timestamp: 1, resultCount: 1 },
      // missing fields
      { query: 'bar' },
      // wrong types
      { query: 42, timestamp: 'now', resultCount: null },
      null,
      'string-not-object',
    ]
    localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(dirty))
    const out = loadRecentSearches()
    expect(out).toHaveLength(1)
    expect(out[0].query).toBe('foo')
  })
})

describe('saveRecentSearches / load round-trip', () => {
  it('persists the array to localStorage under the spec key', () => {
    const items: RecentSearch[] = [
      { query: 'foo', timestamp: 1, resultCount: 0 },
      { query: 'bar', timestamp: 2, resultCount: 5 },
    ]
    saveRecentSearches(items)
    const raw = localStorage.getItem(RECENT_SEARCHES_KEY)
    expect(raw).not.toBeNull()
    expect(JSON.parse(raw!)).toEqual(items)
  })

  it('overwrites any previous value (last writer wins)', () => {
    saveRecentSearches([{ query: 'old', timestamp: 1, resultCount: 0 }])
    saveRecentSearches([{ query: 'new', timestamp: 2, resultCount: 0 }])
    expect(loadRecentSearches()).toEqual([
      { query: 'new', timestamp: 2, resultCount: 0 },
    ])
  })
})

describe('recordRecentSearch', () => {
  it('appends a new search to an empty history', () => {
    const out = recordRecentSearch({ query: 'foo', resultCount: 3 }, now())
    expect(out).toEqual([{ query: 'foo', timestamp: 1_700_000_000_000, resultCount: 3 }])
    expect(loadRecentSearches()).toEqual(out)
  })

  it('moves a repeated query to the front (move-to-top, dedupes)', () => {
    recordRecentSearch({ query: 'foo', resultCount: 1 }, 100)
    recordRecentSearch({ query: 'bar', resultCount: 1 }, 200)
    recordRecentSearch({ query: 'foo', resultCount: 2 }, 300)

    const out = loadRecentSearches()
    expect(out).toHaveLength(2)
    // foo is now first AND it has the latest timestamp + result count.
    expect(out[0]).toEqual({ query: 'foo', timestamp: 300, resultCount: 2 })
    expect(out[1]).toEqual({ query: 'bar', timestamp: 200, resultCount: 1 })
  })

  it('enforces the 10-entry cap (FIFO eviction of the oldest)', () => {
    for (let i = 0; i < 12; i++) {
      recordRecentSearch({ query: `q${i}`, resultCount: 0 }, 1000 + i)
    }
    const out = loadRecentSearches()
    expect(out).toHaveLength(MAX_RECENT_SEARCHES)
    // The two oldest (`q0`, `q1`) were evicted; the newest is first.
    expect(out[0].query).toBe('q11')
    expect(out[MAX_RECENT_SEARCHES - 1].query).toBe('q2')
  })

  it('does not record an empty / whitespace-only query', () => {
    recordRecentSearch({ query: '', resultCount: 0 }, 100)
    recordRecentSearch({ query: '   ', resultCount: 0 }, 200)
    expect(loadRecentSearches()).toEqual([])
  })

  it('treats a trimmed query as the canonical dedup key', () => {
    recordRecentSearch({ query: 'foo', resultCount: 1 }, 100)
    recordRecentSearch({ query: '  foo  ', resultCount: 2 }, 200)
    const out = loadRecentSearches()
    expect(out).toHaveLength(1)
    expect(out[0].query).toBe('foo') // stored trimmed
    expect(out[0].timestamp).toBe(200)
  })
})

// ──── Saved searches ──────────────────────────────────────────────

describe('loadSavedSearches', () => {
  it('returns an empty array when the key is missing', () => {
    expect(loadSavedSearches()).toEqual([])
  })

  it('returns the parsed array on a happy-path read', () => {
    const items: SavedSearch[] = [
      { id: 's1', name: 'Open TODOs', query: 'status:todo', createdAt: 1 },
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(items))
    expect(loadSavedSearches()).toEqual(items)
  })

  it('returns an empty array on malformed JSON', () => {
    localStorage.setItem(SAVED_SEARCHES_KEY, '{not valid')
    expect(loadSavedSearches()).toEqual([])
  })

  it('returns an empty array when the stored value is not an array', () => {
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify('a string'))
    expect(loadSavedSearches()).toEqual([])
  })

  it('filters out entries with missing or wrong-typed fields', () => {
    const dirty = [
      { id: 's1', name: 'Good', query: 'q', createdAt: 1 },
      { id: 42, name: 'Bad id', query: 'q', createdAt: 1 }, // id must be string
      { id: 's2', name: '', query: 'q', createdAt: 1 }, // name must be non-empty
      { id: 's3', name: 'N', query: '', createdAt: 1 }, // query must be non-empty
      { id: 's4', name: 'N', query: 'q', createdAt: 'not-a-number' },
      null,
    ]
    localStorage.setItem(SAVED_SEARCHES_KEY, JSON.stringify(dirty))
    const out = loadSavedSearches()
    expect(out).toHaveLength(1)
    expect(out[0].id).toBe('s1')
  })
})

describe('addSavedSearch', () => {
  it('appends a new saved search to an empty store', () => {
    const added = addSavedSearch(
      { name: 'Open TODOs', query: 'status:todo' },
      () => 'fixed-id-1',
      () => 1234,
    )
    expect(added).toEqual({
      id: 'fixed-id-1',
      name: 'Open TODOs',
      query: 'status:todo',
      createdAt: 1234,
    })
    expect(loadSavedSearches()).toEqual([added])
  })

  it('appends to existing saved searches', () => {
    addSavedSearch({ name: 'A', query: 'a' }, () => 'id-a', () => 1)
    addSavedSearch({ name: 'B', query: 'b' }, () => 'id-b', () => 2)
    const out = loadSavedSearches()
    expect(out.map(s => s.id)).toEqual(['id-a', 'id-b'])
  })

  it('refuses to add a saved search with an empty name', () => {
    expect(() =>
      addSavedSearch({ name: '   ', query: 'q' }, () => 'id', () => 1),
    ).toThrow(/name/i)
  })

  it('refuses to add a saved search with an empty query', () => {
    expect(() =>
      addSavedSearch({ name: 'N', query: '' }, () => 'id', () => 1),
    ).toThrow(/query/i)
  })

  it('persists optional viewType when supplied', () => {
    const added = addSavedSearch(
      { name: 'K', query: 'k', viewType: 'kanban' },
      () => 'k1',
      () => 9,
    )
    expect(added.viewType).toBe('kanban')
    expect(loadSavedSearches()[0].viewType).toBe('kanban')
  })
})

describe('deleteSavedSearch', () => {
  it('removes a saved search by id', () => {
    addSavedSearch({ name: 'A', query: 'a' }, () => 'id-a', () => 1)
    addSavedSearch({ name: 'B', query: 'b' }, () => 'id-b', () => 2)
    addSavedSearch({ name: 'C', query: 'c' }, () => 'id-c', () => 3)

    deleteSavedSearch('id-b')

    const out = loadSavedSearches().map(s => s.id)
    expect(out).toEqual(['id-a', 'id-c'])
  })

  it('is a no-op when the id does not exist', () => {
    addSavedSearch({ name: 'A', query: 'a' }, () => 'id-a', () => 1)
    deleteSavedSearch('does-not-exist')
    expect(loadSavedSearches()).toHaveLength(1)
  })

  it('is a no-op when the store is empty', () => {
    // Just verify it does not throw.
    expect(() => deleteSavedSearch('whatever')).not.toThrow()
  })
})

// ──── Validators (defensive parsing helpers) ──────────────────────

describe('isValidRecentSearch', () => {
  it('accepts a well-formed entry', () => {
    expect(
      isValidRecentSearch({ query: 'foo', timestamp: 1, resultCount: 1 }),
    ).toBe(true)
  })

  it('rejects entries with wrong field types', () => {
    expect(isValidRecentSearch({ query: 1, timestamp: 1, resultCount: 1 })).toBe(false)
    expect(isValidRecentSearch({ query: 'q', timestamp: '1', resultCount: 1 })).toBe(false)
    expect(isValidRecentSearch({ query: 'q', timestamp: 1, resultCount: '1' })).toBe(false)
  })

  it('rejects null and non-objects', () => {
    expect(isValidRecentSearch(null)).toBe(false)
    expect(isValidRecentSearch('string')).toBe(false)
    expect(isValidRecentSearch(undefined)).toBe(false)
  })
})

describe('isValidSavedSearch', () => {
  it('accepts an entry without viewType', () => {
    expect(
      isValidSavedSearch({ id: 's1', name: 'N', query: 'q', createdAt: 1 }),
    ).toBe(true)
  })

  it('accepts an entry with a known viewType', () => {
    expect(
      isValidSavedSearch({ id: 's1', name: 'N', query: 'q', createdAt: 1, viewType: 'kanban' }),
    ).toBe(true)
  })

  it('rejects an unknown viewType (defensive)', () => {
    expect(
      isValidSavedSearch({ id: 's1', name: 'N', query: 'q', createdAt: 1, viewType: 'marquee' }),
    ).toBe(false)
  })

  it('rejects empty name or query', () => {
    expect(
      isValidSavedSearch({ id: 's1', name: '', query: 'q', createdAt: 1 }),
    ).toBe(false)
    expect(
      isValidSavedSearch({ id: 's1', name: 'N', query: '', createdAt: 1 }),
    ).toBe(false)
  })
})
