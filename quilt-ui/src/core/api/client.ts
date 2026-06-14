import type { Block } from '@shared/types/api'
import { blockPropertiesFromMap } from '@shared/utils/blockProperties'

const API_BASE = '/api/v1'
const API_KEY = import.meta.env.VITE_QUILT_API_KEY || ''
const DEFAULT_TTL_MS = 30_000

interface CacheEntry {
  data: unknown
  expireAt: number
}

export interface CachedFetchOptions extends Omit<RequestInit, 'method'> {
  noCache?: boolean
  pageName?: string
}

export interface RawBlock extends Omit<Block, 'properties'> {
  properties?: Record<string, unknown>
}

export function getEventsUrl(): string {
  if (!API_KEY) return '/api/v1/events'
  return `/api/v1/events?api_key=${encodeURIComponent(API_KEY)}`
}

export class QuiltApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    public detail: string,
  ) {
    super(detail)
    this.name = 'QuiltApiError'
  }
}

class SessionCache {
  private pending = new Map<string, Promise<unknown>>()
  private cache = new Map<string, CacheEntry>()
  private pageIndex = new Map<string, Set<string>>()

  get<T>(key: string): T | undefined {
    const entry = this.cache.get(key)
    if (!entry) return undefined
    if (entry.expireAt <= Date.now()) {
      this.cache.delete(key)
      return undefined
    }
    return entry.data as T
  }

  set(key: string, data: unknown, pageName?: string, ttlMs: number = DEFAULT_TTL_MS): void {
    this.cache.set(key, { data, expireAt: Date.now() + ttlMs })
    if (!pageName) return

    let keys = this.pageIndex.get(pageName)
    if (!keys) {
      keys = new Set()
      this.pageIndex.set(pageName, keys)
    }
    keys.add(key)
  }

  invalidate(key: string): void {
    this.cache.delete(key)
    for (const keys of this.pageIndex.values()) {
      keys.delete(key)
    }
  }

  invalidatePage(pageName: string): void {
    const keys = this.pageIndex.get(pageName)
    if (!keys) return
    for (const key of keys) this.cache.delete(key)
    this.pageIndex.delete(pageName)
  }

  invalidateAll(): void {
    this.cache.clear()
    this.pageIndex.clear()
  }

  getOrCreatePending<T>(key: string, factory: () => Promise<T>): Promise<T> {
    const existing = this.pending.get(key)
    if (existing) return existing as Promise<T>

    const created = factory()
    this.pending.set(key, created)
    created.finally(() => this.pending.delete(key)).catch(() => {})
    return created
  }
}

const sessionCache = new SessionCache()

export function createAuthHeaders(): Record<string, string> {
  if (!API_KEY) return {}
  return { Authorization: `Bearer ${API_KEY}` }
}

export async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${url}`, {
    headers: {
      'Content-Type': 'application/json',
      ...createAuthHeaders(),
      ...(options?.headers as Record<string, string> | undefined),
    },
    ...options,
  })

  if (!res.ok) {
    let code = 'INTERNAL_ERROR'
    let detail = res.statusText
    try {
      const body = await res.json()
      code = body.code || code
      detail = body.error || detail
    } catch {
      // ignore parse error
    }
    throw new QuiltApiError(res.status, code, detail)
  }

  if (res.status === 204) return undefined as T
  return res.json()
}

export async function cachedFetch<T>(method: string, url: string, opts: CachedFetchOptions = {}): Promise<T> {
  const isGet = method.toUpperCase() === 'GET'
  const useCache = isGet && !opts.noCache
  const key = `${method.toUpperCase()} ${url}`

  if (useCache) {
    const hit = sessionCache.get<T>(key)
    if (hit !== undefined) return hit

    return sessionCache.getOrCreatePending<T>(key, async () => {
      const data = await fetchJson<T>(url, { ...opts, method })
      sessionCache.set(key, data, opts.pageName)
      return data
    })
  }

  return fetchJson<T>(url, { ...opts, method })
}

export function invalidatePageCache(pageName: string | undefined): void {
  if (!pageName) return
  sessionCache.invalidatePage(pageName)
}

export function invalidateCacheKey(key: string): void {
  sessionCache.invalidate(key)
}

export function invalidateAllCache(): void {
  sessionCache.invalidateAll()
}

export function normalizeBlock(raw: RawBlock): Block {
  return {
    ...raw,
    properties: blockPropertiesFromMap(raw.properties),
  } as Block
}

export function apiBaseUrl(): string {
  return API_BASE
}
