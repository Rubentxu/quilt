/**
 * Tests for api-client — covers fetch-based API calls, error handling,
 * QuiltApiError, auth headers, 204 handling, and block normalization.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { api, QuiltApiError, getEventsUrl } from '@core/api-client'
import type { Block } from '@shared/types/api'

// ── Fetch mock setup ────────────────────────────────────────

const mockFetch = vi.fn()
global.fetch = mockFetch

function mockResponse(status: number, body: unknown, headers: Record<string, string> = {}) {
  mockFetch.mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    statusText: status === 404 ? 'Not Found' : 'OK',
    headers: new Headers(headers),
    json: () => Promise.resolve(body),
  })
}

beforeEach(() => {
  mockFetch.mockReset()
})

// ── QuiltApiError ───────────────────────────────────────────

describe('QuiltApiError', () => {
  it('is an instance of Error', () => {
    const err = new QuiltApiError(404, 'NOT_FOUND', 'Page not found')
    expect(err).toBeInstanceOf(Error)
    expect(err).toBeInstanceOf(QuiltApiError)
  })

  it('has name QuiltApiError', () => {
    const err = new QuiltApiError(500, 'INTERNAL', 'boom')
    expect(err.name).toBe('QuiltApiError')
  })

  it('stores status, code, and detail', () => {
    const err = new QuiltApiError(422, 'BAD_REQUEST', 'Invalid input')
    expect(err.status).toBe(422)
    expect(err.code).toBe('BAD_REQUEST')
    expect(err.detail).toBe('Invalid input')
  })
})

// ── Successful API calls ────────────────────────────────────

describe('listPages', () => {
  it('returns page list on success', async () => {
    const pages = [
      { id: '1', name: 'home', title: 'Home', journal: false, journalDay: null, createdAt: '2026-01-01' },
    ]
    mockResponse(200, pages)

    const result = await api.listPages()
    expect(result).toEqual(pages)
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/pages',
      expect.objectContaining({ headers: expect.any(Object) }),
    )
  })
})

describe('getPage', () => {
  it('returns a single page', async () => {
    const page = { id: '1', name: 'home', title: 'Home', journal: false, journalDay: null, createdAt: '2026-01-01' }
    mockResponse(200, page)

    const result = await api.getPage('home')
    expect(result).toEqual(page)
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/pages/home',
      expect.anything(),
    )
  })

  it('encodes special characters in page name', async () => {
    mockResponse(200, {})
    await api.getPage('my page/with slashes')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/pages/my%20page%2Fwith%20slashes',
      expect.anything(),
    )
  })
})

describe('getPageBlocks', () => {
  it('normalizes block properties from map to array', async () => {
    const rawBlocks = [
      {
        id: 'b1',
        pageId: 'p1',
        pageName: 'test',
        content: 'Hello',
        blockType: 'paragraph',
        marker: null,
        priority: null,
        parentId: null,
        order: 0,
        level: 1,
        collapsed: false,
        properties: { status: 'draft', count: 5 },
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
    ]
    mockResponse(200, rawBlocks)

    const result = await api.getPageBlocks('test')
    expect(result).toHaveLength(1)
    // Properties should be normalized to BlockProperty[]
    expect(result[0].properties).toEqual([
      { key: 'status', value: 'draft', type: 'string' },
      { key: 'count', value: 5, type: 'number' },
    ])
  })
})

// ── Error handling ──────────────────────────────────────────

describe('error handling', () => {
  it('throws QuiltApiError on non-ok response', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
      statusText: 'Not Found',
      json: () => Promise.resolve({ code: 'NOT_FOUND', error: 'Page not found' }),
    })

    await expect(api.getPage('nonexistent')).rejects.toThrow(QuiltApiError)
  })

  it('includes error code and detail in QuiltApiError', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 422,
      statusText: 'Unprocessable',
      json: () => Promise.resolve({ code: 'BAD_REQUEST', error: 'Invalid name' }),
    })

    try {
      await api.createPage({ name: 'bad//name' })
      expect.fail('should have thrown')
    } catch (err) {
      expect(err).toBeInstanceOf(QuiltApiError)
      expect((err as QuiltApiError).status).toBe(422)
      expect((err as QuiltApiError).code).toBe('BAD_REQUEST')
    }
  })

  it('falls back to statusText when body is not JSON', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
      json: () => Promise.reject(new Error('not json')),
    })

    try {
      await api.getPage('test')
      expect.fail('should have thrown')
    } catch (err) {
      expect(err).toBeInstanceOf(QuiltApiError)
      expect((err as QuiltApiError).status).toBe(500)
    }
  })
})

// ── POST / PATCH / DELETE ───────────────────────────────────

describe('mutations', () => {
  it('createPage sends POST with body', async () => {
    const page = { id: 'new', name: 'new-page', title: null, journal: false, journalDay: null, createdAt: '2026-01-01' }
    mockResponse(201, page)

    const result = await api.createPage({ name: 'new-page' })
    expect(result).toEqual(page)
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/pages',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ name: 'new-page' }),
      }),
    )
  })

  it('deleteBlock sends DELETE', async () => {
    mockResponse(200, { deleted: true })

    const result = await api.deleteBlock('b1')
    expect(result).toEqual({ deleted: true })
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/blocks/b1',
      expect.objectContaining({ method: 'DELETE' }),
    )
  })
})

// ── 204 No Content ──────────────────────────────────────────

describe('204 handling', () => {
  it('returns undefined for 204 responses', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      statusText: 'No Content',
      json: () => Promise.reject(new Error('no body')),
    })

    // setBlockProperty returns fetchJson<void>, 204 means success
    const result = await api.setBlockProperty('b1', 'key', 'value')
    expect(result).toBeUndefined()
  })
})

// ── getEventsUrl — SSE auth (F4 of quilt-fase2-ux-dead-buttons) ──
//
// The browser's EventSource cannot set custom request headers, so
// the token is passed as `?api_key=...` and the server accepts the
// query param only on `/api/v1/events`.
//
// The Vite env var is read at module-load time, so we exercise both
// branches by importing a fresh copy of the module after stubbing
// `import.meta.env`. Vitest's `vi.resetModules()` + dynamic
// `await import(...)` does the trick.

describe('getEventsUrl', () => {
  it('appends ?api_key=<token> when VITE_QUILT_API_KEY is set', async () => {
    vi.resetModules()
    vi.stubEnv('VITE_QUILT_API_KEY', 'test-token-abc')
    const mod = await import('@core/api-client')
    expect(mod.getEventsUrl()).toBe('/api/v1/events?api_key=test-token-abc')
    vi.unstubAllEnvs()
  })

  it('URL-encodes the token to handle special characters', async () => {
    vi.resetModules()
    vi.stubEnv('VITE_QUILT_API_KEY', 'key with spaces/and/slashes=and=equals')
    const mod = await import('@core/api-client')
    const url = mod.getEventsUrl()
    // Token must be percent-encoded so it survives the URL parser
    // unchanged when the server's `?split('&').find_map(strip_prefix)`
    // pulls it back out.
    expect(url).toBe(
      '/api/v1/events?api_key=key%20with%20spaces%2Fand%2Fslashes%3Dand%3Dequals',
    )
    vi.unstubAllEnvs()
  })

  it('returns plain /api/v1/events when no API key is configured', async () => {
    vi.resetModules()
    vi.stubEnv('VITE_QUILT_API_KEY', '')
    const mod = await import('@core/api-client')
    expect(mod.getEventsUrl()).toBe('/api/v1/events')
    vi.unstubAllEnvs()
  })
})
