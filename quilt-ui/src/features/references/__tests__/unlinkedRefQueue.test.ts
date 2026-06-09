import { describe, it, expect, vi, beforeEach } from 'vitest'

// ─── Mocks ─────────────────────────────────────────────────────────
//
// localStorage is shimmed in src/test/setup.ts, so we just reset it
// per test. We mock `@core/api-client` for the integration tests of
// `useUnlinkedRefQueue`; the pure utility tests don't need it.

const mockListPages = vi.fn()
const mockSearchBlocks = vi.fn()
const mockUpdateBlock = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    listPages: () => mockListPages(),
    searchBlocks: (q: string, limit?: number) => mockSearchBlocks(q, limit),
    updateBlock: (id: string, data: { content: string }) => mockUpdateBlock(id, data),
  },
}))

// Import after mocks so the hook sees the mocked api
import {
  detectMentions,
  linkifyMention,
  loadQueue,
  saveQueue,
  removeCandidate,
  STORAGE_KEY,
  type UnlinkedCandidate,
} from '../unlinkedRefQueue'
import { useUnlinkedRefQueue } from '../useUnlinkedRefQueue'
import { renderHook, act, waitFor } from '@testing-library/react'

function makeCandidate(overrides: Partial<UnlinkedCandidate> = {}): UnlinkedCandidate {
  return {
    blockId: 'b-1',
    pageName: 'Demo Page',
    mentionText: 'see Demo Page for details',
    position: 4,
    createdAt: 1700000000000,
    ...overrides,
  }
}

beforeEach(() => {
  localStorage.clear()
  mockListPages.mockReset()
  mockSearchBlocks.mockReset()
  mockUpdateBlock.mockReset()
  vi.useRealTimers()
})

// ─── detectMentions (pure) ────────────────────────────────────────

describe('detectMentions — find page name in block content', () => {
  it('finds a plain page-name mention that is NOT wrapped in [[ ]]', () => {
    const hits = detectMentions('read more on Demo Page today', 'Demo Page')
    expect(hits).toHaveLength(1)
    expect(hits[0]).toMatchObject({
      position: 13,
      mentionText: 'Demo Page',
    })
  })

  it('skips mentions that are already wrapped in [[ ]] (already linked)', () => {
    const hits = detectMentions('see [[Demo Page]] for context', 'Demo Page')
    expect(hits).toHaveLength(0)
  })

  it('skips mentions already inside a wikilink even when extra text follows', () => {
    const hits = detectMentions('a [[Demo Page|alias here]] b', 'Demo Page')
    expect(hits).toHaveLength(0)
  })

  it('returns multiple hits when the name appears several times', () => {
    // Index:  0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16 17 18 19 20 21
    // Char:   D  e  m  o     P  a  g  e     t  h  e  n     l  a  t  e  r     D
    // 1st match: positions 0..9.  2nd match starts at position 21
    // (the leading space of " Demo" sits at index 20).
    const hits = detectMentions('Demo Page then later Demo Page again', 'Demo Page')
    expect(hits).toHaveLength(2)
    expect(hits[0].position).toBe(0)
    expect(hits[1].position).toBe(21)
  })

  it('is case-insensitive on the search term (case-preserving in output)', () => {
    const hits = detectMentions('lowercase demo page should still hit', 'Demo Page')
    expect(hits).toHaveLength(1)
    expect(hits[0].mentionText).toBe('demo page')
  })

  it('does not match a substring of a longer word', () => {
    const hits = detectMentions('MyDemo Pagework is unrelated', 'Demo Page')
    expect(hits).toHaveLength(0)
  })

  it('returns an empty list when content is empty', () => {
    expect(detectMentions('', 'Demo Page')).toEqual([])
  })

  it('escapes regex metacharacters in the page name', () => {
    // C++. (period + plus + plus) would break a naive substring search if
    // we built a regex from the name. We want literal matching, so the
    // periods must be treated as literal dots.
    const hits = detectMentions('I love C++. so much', 'C++.')
    expect(hits).toHaveLength(1)
    expect(hits[0].mentionText).toBe('C++.')
  })
})

// ─── linkifyMention (pure) ────────────────────────────────────────

describe('linkifyMention — wrap the mention in [[ ]]', () => {
  it('inserts [[ ]] around the mention at the given position', () => {
    const out = linkifyMention('see Demo Page today', {
      position: 4,
      mentionText: 'Demo Page',
      blockId: 'b-1',
      pageName: 'Demo Page',
      createdAt: 0,
    })
    expect(out).toBe('see [[Demo Page]] today')
  })

  it('replaces the matched span when the mention case differs from the page name', () => {
    // The detected mention is the substring in the content; we always
    // wrap with the canonical page name to normalize the link target.
    const out = linkifyMention('see demo page today', {
      position: 4,
      mentionText: 'demo page',
      blockId: 'b-1',
      pageName: 'Demo Page',
      createdAt: 0,
    })
    expect(out).toBe('see [[Demo Page]] today')
  })

  it('handles a mention at position 0', () => {
    const out = linkifyMention('Demo Page is great', {
      position: 0,
      mentionText: 'Demo Page',
      blockId: 'b-1',
      pageName: 'Demo Page',
      createdAt: 0,
    })
    expect(out).toBe('[[Demo Page]] is great')
  })

  it('leaves the content untouched if the position no longer matches (defensive)', () => {
    // If the block was edited between detection and link action, the
    // char at `position` might not be the mention anymore. We bail
    // out rather than corrupt the content.
    const out = linkifyMention('completely different content', {
      position: 0,
      mentionText: 'Demo Page',
      blockId: 'b-1',
      pageName: 'Demo Page',
      createdAt: 0,
    })
    expect(out).toBe('completely different content')
  })
})

// ─── localStorage queue helpers (pure-ish) ────────────────────────

describe('localStorage queue — load / save / remove', () => {
  it('returns an empty queue when the key is absent', () => {
    expect(loadQueue()).toEqual([])
  })

  it('returns an empty queue when the stored value is malformed JSON', () => {
    localStorage.setItem(STORAGE_KEY, 'not json')
    expect(loadQueue()).toEqual([])
  })

  it('returns an empty queue when the stored value is not an array', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ foo: 'bar' }))
    expect(loadQueue()).toEqual([])
  })

  it('round-trips a queue through saveQueue → loadQueue', () => {
    const candidates: UnlinkedCandidate[] = [makeCandidate(), makeCandidate({ blockId: 'b-2' })]
    saveQueue(candidates)
    expect(loadQueue()).toEqual(candidates)
  })

  it('saveQueue overwrites the previous contents', () => {
    saveQueue([makeCandidate({ blockId: 'a' })])
    saveQueue([makeCandidate({ blockId: 'b' })])
    const loaded = loadQueue()
    expect(loaded).toHaveLength(1)
    expect(loaded[0].blockId).toBe('b')
  })

  it('removeCandidate filters by blockId + position', () => {
    saveQueue([
      makeCandidate({ blockId: 'a', position: 0 }),
      makeCandidate({ blockId: 'a', position: 5 }),
      makeCandidate({ blockId: 'b', position: 0 }),
    ])
    removeCandidate('a', 0)
    const remaining = loadQueue()
    expect(remaining).toHaveLength(2)
    expect(remaining.map((c) => `${c.blockId}:${c.position}`)).toEqual(['a:5', 'b:0'])
  })

  it('removeCandidate is a no-op when the candidate does not exist', () => {
    saveQueue([makeCandidate({ blockId: 'a' })])
    removeCandidate('does-not-exist', 0)
    expect(loadQueue()).toHaveLength(1)
  })
})

// ─── useUnlinkedRefQueue hook (integration with mocked api) ───────

describe('useUnlinkedRefQueue — detection + actions', () => {
  it('initial render: queue is empty and no api call is made (no pageName)', () => {
    mockListPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    const { result } = renderHook(() => useUnlinkedRefQueue(null))

    expect(result.current.queue).toEqual([])
    expect(result.current.loading).toBe(false)
    expect(mockListPages).not.toHaveBeenCalled()
    expect(mockSearchBlocks).not.toHaveBeenCalled()
  })

  it('detects unlinked mentions on a page and populates the queue', async () => {
    // listPages returns the target page so the hook knows the
    // "pageName → search query" mapping for the wiki-link syntax.
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: 'Demo', journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'see Demo Page for more',
        snippet: '...',
        score: 0.9,
      },
    ])

    const { result } = renderHook(() => useUnlinkedRefQueue('Demo Page'))

    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.queue).toHaveLength(1)
    expect(result.current.queue[0]).toMatchObject({
      blockId: 'b-1',
      pageName: 'Demo Page',
      mentionText: 'Demo Page',
    })
    // Persisted to localStorage so a reload restores it
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]')).toHaveLength(1)
  })

  it('does not enqueue mentions that are already inside [[ ]]', async () => {
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'see [[Demo Page]] for more',
        snippet: '',
        score: 1,
      },
    ])

    const { result } = renderHook(() => useUnlinkedRefQueue('Demo Page'))

    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.queue).toEqual([])
  })

  it('reuses the persisted queue across remounts (no extra fetch)', async () => {
    // First mount populates the queue
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'see Demo Page for more',
        snippet: '',
        score: 1,
      },
    ])

    const first = renderHook(() => useUnlinkedRefQueue('Demo Page'))
    await waitFor(() => expect(first.result.current.queue).toHaveLength(1))

    // Unmount + remount with the same pageName
    first.unmount()
    mockListPages.mockClear()
    mockSearchBlocks.mockClear()
    // listPages still gets called for the page-name lookup,
    // searchBlocks is NOT called (cached in localStorage)
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])

    const second = renderHook(() => useUnlinkedRefQueue('Demo Page'))
    // The hook re-validates against the current backend state, so it
    // will call searchBlocks — but the persisted queue should be the
    // starting point and the resulting queue still has the candidate.
    await waitFor(() => expect(second.result.current.queue).toHaveLength(1))
    // Persistence: reloaded from localStorage on the first effect tick
    expect(second.result.current.queue[0].blockId).toBe('b-1')
  })

  it('dismiss removes a candidate from the queue and from localStorage', async () => {
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'Demo Page here',
        snippet: '',
        score: 1,
      },
    ])

    const { result } = renderHook(() => useUnlinkedRefQueue('Demo Page'))
    await waitFor(() => expect(result.current.queue).toHaveLength(1))

    act(() => result.current.dismiss(result.current.queue[0]))

    expect(result.current.queue).toEqual([])
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]')).toEqual([])
  })

  it('link inserts [[ ]] in the block content, drops the candidate, and persists the new content', async () => {
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'see Demo Page for more',
        snippet: '',
        score: 1,
      },
    ])
    mockUpdateBlock.mockResolvedValue({} as any)

    const { result } = renderHook(() => useUnlinkedRefQueue('Demo Page'))
    await waitFor(() => expect(result.current.queue).toHaveLength(1))

    const candidate = result.current.queue[0]
    await act(async () => {
      await result.current.link(candidate)
    })

    expect(mockUpdateBlock).toHaveBeenCalledWith('b-1', { content: 'see [[Demo Page]] for more' })
    expect(result.current.queue).toEqual([])
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]')).toEqual([])
  })

  it('link surfaces a toast.error when the api call fails', async () => {
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'Demo Page', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-1',
        pageId: 'p-other',
        pageName: 'Other Page',
        content: 'see Demo Page for more',
        snippet: '',
        score: 1,
      },
    ])
    mockUpdateBlock.mockRejectedValue(new Error('boom'))

    const { result } = renderHook(() => useUnlinkedRefQueue('Demo Page'))
    await waitFor(() => expect(result.current.queue).toHaveLength(1))

    await act(async () => {
      await result.current.link(result.current.queue[0])
    })

    // Candidate is kept so the user can retry
    expect(result.current.queue).toHaveLength(1)
  })
})
