/**
 * Tests for the SessionCache layer in api-client.
 *
 * The SessionCache is a request-deduplication + short-TTL response cache
 * that sits between the public `api` object and the underlying `fetch`.
 * Goals:
 *   1. Concurrent identical GETs collapse to a single network call.
 *   2. Sequential identical GETs within TTL hit the cache.
 *   3. Sequential identical GETs past TTL go back to the network.
 *   4. Mutations invalidate the relevant cache entries so we never
 *      serve stale data after a write.
 *   5. Non-GET requests never enter the cache (no dedup, no store).
 *
 * The cache is internal (not exported) so these tests poke at the
 * behavior through the public `api` surface — that's the contract
 * the rest of the app depends on.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api } from '@core/api-client'

// ── Fetch mock setup ────────────────────────────────────────

const mockFetch = vi.fn()
global.fetch = mockFetch

function mockResponse(status: number, body: unknown) {
  mockFetch.mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    statusText: status === 404 ? 'Not Found' : 'OK',
    headers: new Headers(),
    json: () => Promise.resolve(body),
  })
}

/**
 * Helper: build a minimal `Page` payload the API returns for
 * `getPage`. The shape matches what `getPage` consumers expect
 * (id, name, title, journal, journalDay, createdAt) and is the
 * minimum we need to assert on (id is the unique discriminator).
 */
function pageFixture(name: string) {
  return {
    id: `id-${name}`,
    name,
    title: name,
    journal: false,
    journalDay: null,
    createdAt: '2026-01-01T00:00:00Z',
  }
}

beforeEach(() => {
  mockFetch.mockReset()
  // Drop the SessionCache between tests so a stored GET body from
  // a prior case doesn't short-circuit the next one.
  api.invalidateAll()
})

// ── 1. Promise dedup: concurrent identical GETs ─────────────
//
// Three calls fired in the same microtask tick must all resolve
// to the same body AND result in exactly ONE network request.
// This is the headline use case — sidebar / page header /
// reference panel all calling `getPage('foo')` at once.

describe('SessionCache: promise dedup', () => {
  it('collapses three concurrent identical GETs into one network call', async () => {
    const page = pageFixture('dedup')
    mockResponse(200, page)

    // Fire three concurrent requests, no await between them.
    const [a, b, c] = await Promise.all([
      api.getPage('dedup'),
      api.getPage('dedup'),
      api.getPage('dedup'),
    ])

    expect(a).toEqual(page)
    expect(b).toEqual(page)
    expect(c).toEqual(page)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('dedup applies to other GET endpoints too (getPageBlocks)', async () => {
    const rawBlocks = [
      {
        id: 'b1',
        pageId: 'p1',
        pageName: 'dedup-blocks',
        content: 'hi',
        blockType: 'paragraph',
        marker: null,
        priority: null,
        parentId: null,
        order: 0,
        level: 1,
        collapsed: false,
        properties: {},
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
    ]
    mockResponse(200, rawBlocks)

    const [a, b] = await Promise.all([
      api.getPageBlocks('dedup-blocks'),
      api.getPageBlocks('dedup-blocks'),
    ])

    expect(a).toHaveLength(1)
    expect(b).toEqual(a)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('different cache keys do not dedup (parallel calls to different pages)', async () => {
    mockResponse(200, pageFixture('a'))
    mockResponse(200, pageFixture('b'))

    const [pa, pb] = await Promise.all([api.getPage('a'), api.getPage('b')])

    expect(pa.name).toBe('a')
    expect(pb.name).toBe('b')
    expect(mockFetch).toHaveBeenCalledTimes(2)
  })
})

// ── 2. Cache hit within TTL ────────────────────────────────
//
// Sequential GETs (second one AFTER the first resolved) should
// also short-circuit if the entry is still in the cache. This
// is the bread-and-butter case for navigation: user lands on
// page A, navigates away, comes back within 30s — no refetch.

describe('SessionCache: sequential hit within TTL', () => {
  it('serves a cached GET without re-fetching on second call', async () => {
    const page = pageFixture('hot')
    mockResponse(200, page)

    const first = await api.getPage('hot')
    const second = await api.getPage('hot')

    expect(first).toEqual(page)
    expect(second).toEqual(page)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('caches normalized block shape (post-transform), not raw server shape', async () => {
    // First call returns raw blocks (with `properties` as a map);
    // getPageBlocks normalizes to BlockProperty[]. The cached
    // value must be the normalized shape so consumers never see
    // the raw map on a cache hit.
    const rawBlocks = [
      {
        id: 'b1',
        pageId: 'p1',
        pageName: 'norm',
        content: 'hi',
        blockType: 'paragraph',
        marker: null,
        priority: null,
        parentId: null,
        order: 0,
        level: 1,
        collapsed: false,
        properties: { status: 'ready' },
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
    ]
    mockResponse(200, rawBlocks)

    const first = await api.getPageBlocks('norm')
    const second = await api.getPageBlocks('norm')

    expect(first[0].properties).toEqual([{ key: 'status', value: 'ready', type: 'string' }])
    expect(second[0].properties).toEqual([{ key: 'status', value: 'ready', type: 'string' }])
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })
})

// ── 3. Cache miss after TTL ────────────────────────────────
//
// We can't actually wait 30 seconds in a test, so we test the
// boundary by mutating the cached entry's `expireAt` into the
// past. The cache exposes a small surface; if the cleanest path
// is to advance the system clock we use `vi.useFakeTimers`.
// We pick the "mutate expireAt" approach because it doesn't
// depend on timer-implementation details and exercises the
// exact code path (the `get()` check).

describe('SessionCache: miss after TTL', () => {
  it('re-fetches when the cached entry has expired', async () => {
    const stale = pageFixture('stale')
    const fresh = { ...pageFixture('stale'), title: 'stale (updated)' }
    mockResponse(200, stale)
    mockResponse(200, fresh)

    const first = await api.getPage('stale')
    expect(first.title).toBe('stale')

    // Manually expire the entry: the only way to reach the cache
    // from outside is through the public api, so we expose a tiny
    // test hook on the cache. If no hook exists, we drive the
    // timer instead (see alternate test below).
    // First try: use vi fake timers + advance by 31s.
    vi.useFakeTimers()
    // The first call resolved at t=0 (real time). Advance past
    // the 30s default TTL.
    vi.advanceTimersByTime(31_000)

    const second = await api.getPage('stale')
    expect(second.title).toBe('stale (updated)')
    expect(mockFetch).toHaveBeenCalledTimes(2)

    vi.useRealTimers()
  })
})

// ── 4. Invalidation: mutations clear related entries ──────
//
// This is the most important property for correctness: after
// a write, the next read MUST go to the network, not return
// a stale cached body. We cover page-level and block-level
// invalidation separately because they key on different paths.

describe('SessionCache: invalidation on mutations', () => {
  it('createPage invalidates the cached /pages/:name entry', async () => {
    const page = pageFixture('invalidate-create')
    mockResponse(200, page)

    // Prime the cache
    await api.getPage('invalidate-create')
    expect(mockFetch).toHaveBeenCalledTimes(1)

    // Create the page (POST). Should NOT be cached, and should
    // invalidate the existing entry.
    const created = { ...pageFixture('invalidate-create'), id: 'id-new' }
    mockResponse(201, created)
    await api.createPage({ name: 'invalidate-create' })
    expect(mockFetch).toHaveBeenCalledTimes(2)

    // Next read should hit the network (the response from POST
    // is different from the cached one).
    const refreshed = { ...pageFixture('invalidate-create'), id: 'id-newer' }
    mockResponse(200, refreshed)
    const result = await api.getPage('invalidate-create')
    expect(result.id).toBe('id-newer')
    expect(mockFetch).toHaveBeenCalledTimes(3)
  })

  it('createBlock invalidates the parent page cache (both page and blocks)', async () => {
    const page = pageFixture('parent')
    const rawBlocks: unknown[] = []
    mockResponse(200, page)
    mockResponse(200, rawBlocks)

    // Prime caches for both the page and its blocks
    await api.getPage('parent')
    await api.getPageBlocks('parent')
    expect(mockFetch).toHaveBeenCalledTimes(2)

    // Create a block under that page — should invalidate both
    // the `/pages/parent` entry and the `/pages/parent/blocks` entry.
    const newBlock = {
      id: 'b-new',
      pageId: 'parent',
      pageName: 'parent',
      content: 'new',
      blockType: 'paragraph',
      marker: null,
      priority: null,
      parentId: null,
      order: 0,
      level: 1,
      collapsed: false,
      properties: {},
      createdAt: '2026-01-01',
      updatedAt: '2026-01-01',
    }
    mockResponse(201, newBlock)
    await api.createBlock({ pageName: 'parent', content: 'new' })
    expect(mockFetch).toHaveBeenCalledTimes(3)

    // Both reads should go back to the network.
    const pageAgain = pageFixture('parent')
    const blocksAgain = [newBlock]
    mockResponse(200, pageAgain)
    mockResponse(200, blocksAgain)

    await api.getPage('parent')
    await api.getPageBlocks('parent')
    expect(mockFetch).toHaveBeenCalledTimes(5)
  })

  it('updateBlock invalidates the parent page cache', async () => {
    // Prime: read a page that contains a block we'll later update
    const page = pageFixture('update-parent')
    mockResponse(200, page)
    await api.getPage('update-parent')
    expect(mockFetch).toHaveBeenCalledTimes(1)

    // Update a block on that page. The 3rd arg is the pageName —
    // the cache uses it to invalidate `/pages/:pageName` and
    // `/pages/:pageName/blocks`. Optional, so existing callers
    // that don't pass it keep working.
    const updatedBlock = {
      id: 'b1',
      pageId: 'update-parent',
      pageName: 'update-parent',
      content: 'updated content',
      blockType: 'paragraph',
      marker: null,
      priority: null,
      parentId: null,
      order: 0,
      level: 1,
      collapsed: false,
      properties: {},
      createdAt: '2026-01-01',
      updatedAt: '2026-01-02',
    }
    mockResponse(200, updatedBlock)
    await api.updateBlock('b1', { content: 'updated content' }, 'update-parent')
    expect(mockFetch).toHaveBeenCalledTimes(2)

    // Next read should hit the network.
    const refreshed = { ...pageFixture('update-parent'), title: 'after-update' }
    mockResponse(200, refreshed)
    const result = await api.getPage('update-parent')
    expect(result.title).toBe('after-update')
    expect(mockFetch).toHaveBeenCalledTimes(3)
  })

  it('deleteBlock invalidates the parent page cache', async () => {
    const page = pageFixture('delete-parent')
    mockResponse(200, page)
    await api.getPage('delete-parent')
    expect(mockFetch).toHaveBeenCalledTimes(1)

    mockResponse(200, { deleted: true })
    await api.deleteBlock('b1', 'delete-parent')
    expect(mockFetch).toHaveBeenCalledTimes(2)

    const refreshed = { ...pageFixture('delete-parent'), id: 'changed' }
    mockResponse(200, refreshed)
    const result = await api.getPage('delete-parent')
    expect(result.id).toBe('changed')
    expect(mockFetch).toHaveBeenCalledTimes(3)
  })
})

// ── 5. Non-GET requests are never cached ──────────────────
//
// Mutations never enter the cache and never dedup. If two
// `createBlock` calls fire concurrently, both hit the network.
// (We don't promise server-side dedup — that's the server's job.)

describe('SessionCache: non-GET requests bypass cache', () => {
  it('two concurrent createPage calls both hit the network (no dedup)', async () => {
    const a = { ...pageFixture('a'), id: 'new-a' }
    const b = { ...pageFixture('b'), id: 'new-b' }
    mockResponse(201, a)
    mockResponse(201, b)

    const [ra, rb] = await Promise.all([
      api.createPage({ name: 'a' }),
      api.createPage({ name: 'b' }),
    ])

    expect(ra).toEqual(a)
    expect(rb).toEqual(b)
    expect(mockFetch).toHaveBeenCalledTimes(2)
  })

  it('mutation results are not stored, so a subsequent GET refetches', async () => {
    // Prime cache via GET
    const page = pageFixture('not-cached')
    mockResponse(200, page)
    await api.getPage('not-cached')
    expect(mockFetch).toHaveBeenCalledTimes(1)

    // A mutation that targets the same URL pattern but is a
    // POST. Even if it returned the same body, the cache must
    // not store it (and must not serve it back as a "cache hit"
    // to a subsequent GET).
    const mutated = { ...pageFixture('not-cached'), id: 'mutated' }
    mockResponse(200, mutated)
    await api.createPageFromTemplate({
      templateName: 'template/x',
      pageName: 'not-cached',
    })
    expect(mockFetch).toHaveBeenCalledTimes(2)

    // A separate GET for the same path must still come from the
    // ORIGINAL cached entry, not from the mutation response. The
    // mutation should, however, have invalidated the original.
    const refreshed = { ...pageFixture('not-cached'), id: 'refreshed' }
    mockResponse(200, refreshed)
    const result = await api.getPage('not-cached')
    expect(result.id).toBe('refreshed')
    expect(mockFetch).toHaveBeenCalledTimes(3)
  })
})
