import type {
  Page,
  Block,
  BlockProperty,
  CreatePageRequest,
  CreateBlockRequest,
  CreatePageFromTemplateRequest,
  CreatePageFromTemplateResponse,
  UpdateBlockRequest,
  UserSettings,
  UpdateSettingsRequest,
  DateFormatOption,
  Backlink,
  SearchResult,
  TemplateSummary,
  TemplateSchema,
  TourStateResponse,
  DismissTourRequest,
} from '@shared/types/api';
import type { QueryAst, QueryError, QueryResult } from '@shared/types/queryAst';
import { blockPropertiesFromMap } from '@shared/utils/blockProperties';

const API_BASE = '/api/v1';

/** Auth token loaded from environment — all API calls include `Authorization: Bearer <token>` */
const API_KEY = import.meta.env.VITE_QUILT_API_KEY || '';

/**
 * Returns the full URL for the SSE events endpoint, with the API
 * key passed as a `?api_key=` query parameter when one is configured.
 *
 * Why a query param and not a header? The browser's `EventSource`
 * API has no API to set custom request headers, so SSE must pass
 * the token in the URL. The server's auth middleware accepts the
 * `api_key` query param only on `/api/v1/events` — every other
 * `/api/v1/*` route still requires the `Authorization: Bearer`
 * header.
 *
 * Exported separately (rather than built inside the `api` object) so
 * `useSSE` can be a generic hook that doesn't have to import the
 * whole client. Returns the plain `/api/v1/events` URL when no API
 * key is set (e.g. local dev with auth disabled).
 */
export function getEventsUrl(): string {
  if (!API_KEY) return '/api/v1/events'
  return `/api/v1/events?api_key=${encodeURIComponent(API_KEY)}`
}

// ──── Error class ───────────────────────────────────────────────

export class QuiltApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    public detail: string
  ) {
    super(detail);
    this.name = 'QuiltApiError';
  }
}

// ──── SessionCache ──────────────────────────────────────────────
//
// In-process request-deduplication + short-TTL response cache for GETs.
// Two responsibilities:
//
//   1. Promise dedup — if `getPage('x')` is called N times before the
//      first response resolves, only ONE network call fires. The other
//      N-1 callers receive the same Promise (and thus the same result
//      or rejection).
//
//   2. TTL cache — once a GET resolves, its body is stashed for
//      `DEFAULT_TTL_MS` (30s). Subsequent identical GETs within that
//      window short-circuit straight to the cached body.
//
// Mutations (`createPage`, `createBlock`, `updateBlock`, `deleteBlock`,
// `updateSettings`, `dismissTour`, …) bypass the cache entirely AND
// invalidate the affected page's entries so we never serve stale data
// after a write. The 30s TTL is short on purpose — defense in depth.
//
// The cache is process-local (one per browser tab), in-memory, and
// not persisted across reloads. A new tab starts cold; an explicit
// `api.invalidateAll()` exists for tests and for use cases that
// require it (e.g. SSE-driven "graph mutated" event).

const DEFAULT_TTL_MS = 30_000;

interface CacheEntry {
  data: unknown;
  expireAt: number;
}

interface CachedFetchOptions extends Omit<RequestInit, 'method'> {
  /**
   * Per-call escape hatch: when truthy, the request bypasses both the
   * pending-Promise dedup and the TTL cache. Used by callers that
   * pass `Cache-Control: no-cache` to express "always go to the
   * network" (e.g. settings that opt out of caching).
   */
  noCache?: boolean;
  /**
   * When set, the cached entry is registered under this page in the
   * secondary index. Mutations targeting the same page can then drop
   * the entry without scanning the whole cache.
   */
  pageName?: string;
}

class SessionCache {
  /** In-flight GETs keyed by `${METHOD} ${url}`. */
  private pending = new Map<string, Promise<unknown>>();
  /** Resolved GET bodies keyed by `${METHOD} ${url}`, with TTL. */
  private cache = new Map<string, CacheEntry>();
  /**
   * Secondary index: pageName → set of cache keys belonging to that
   * page. Lets mutations invalidate everything for a page in O(1)
   * (per key) instead of scanning the whole cache.
   */
  private pageIndex = new Map<string, Set<string>>();

  /**
   * Look up a cached body. Returns `undefined` on miss or expiry.
   * Expired entries are removed lazily — we never need a timer.
   */
  get<T>(key: string): T | undefined {
    const entry = this.cache.get(key);
    if (!entry) return undefined;
    if (entry.expireAt <= Date.now()) {
      this.cache.delete(key);
      return undefined;
    }
    return entry.data as T;
  }

  /** Store a body with the default TTL and register in the page index. */
  set(key: string, data: unknown, pageName?: string, ttlMs: number = DEFAULT_TTL_MS): void {
    this.cache.set(key, { data, expireAt: Date.now() + ttlMs });
    if (pageName) {
      let keys = this.pageIndex.get(pageName);
      if (!keys) {
        keys = new Set();
        this.pageIndex.set(pageName, keys);
      }
      keys.add(key);
    }
  }

  /** Drop a single cache key. */
  invalidate(key: string): void {
    this.cache.delete(key);
    // Also clean the secondary index: the key no longer belongs to any page.
    for (const keys of this.pageIndex.values()) {
      keys.delete(key);
    }
  }

  /** Drop every cache key registered under a given page. */
  invalidatePage(pageName: string): void {
    const keys = this.pageIndex.get(pageName);
    if (!keys) return;
    for (const key of keys) this.cache.delete(key);
    this.pageIndex.delete(pageName);
  }

  /**
   * Drop every cache entry. Used by tests and by callers that need to
   * force a cold start (e.g. after detecting a graph-wide mutation
   * via SSE).
   */
  invalidateAll(): void {
    this.cache.clear();
    this.pageIndex.clear();
    // NOTE: do NOT clear `pending` — those are in-flight network
    // requests, not stale data. Letting them complete is correct.
  }

  /**
   * Internal: register or retrieve the in-flight Promise for a key.
   * The caller is responsible for actually firing the network request
   * when no Promise exists yet.
   */
  getOrCreatePending<T>(key: string, factory: () => Promise<T>): Promise<T> {
    const existing = this.pending.get(key);
    if (existing) return existing as Promise<T>;
    const created = factory();
    this.pending.set(key, created);
    // Always clean up the pending entry when the request settles,
    // success or failure, so the next call starts a fresh request.
    // The empty `.catch(() => {})` here is intentional: it prevents
    // `Promise.prototype.finally` from synthesizing a NEW rejected
    // promise (which Node would log as an unhandled rejection when
    // the original request fails). The caller's own `.catch` /
    // `try/await` still sees the original error unchanged.
    created.finally(() => this.pending.delete(key)).catch(() => {});
    return created;
  }
}

/** Single shared cache for the lifetime of the page. */
const sessionCache = new SessionCache();

// ──── Fetch helper ──────────────────────────────────────────────

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const authHeaders: Record<string, string> = {};
  if (API_KEY) {
    authHeaders['Authorization'] = `Bearer ${API_KEY}`;
  }

  const res = await fetch(`${API_BASE}${url}`, {
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders,
      ...(options?.headers as Record<string, string> | undefined),
    },
    ...options,
  });

  if (!res.ok) {
    let code = 'INTERNAL_ERROR';
    let detail = res.statusText;
    try {
      const body = await res.json();
      code = body.code || code;
      detail = body.error || detail;
    } catch {
      // ignore parse error
    }
    throw new QuiltApiError(res.status, code, detail);
  }

  // Handle 204 No Content
  if (res.status === 204) return undefined as T;

  return res.json();
}

/**
 * Cache-aware wrapper around `fetchJson`. Only GETs participate in
 * the cache. The cache key is `${method} ${url}` (no body for GETs
 * in this codebase). When `noCache` is set, the request bypasses
 * the cache entirely — the result is still returned, just not
 * deduped or stored.
 */
async function cachedFetch<T>(method: string, url: string, opts: CachedFetchOptions = {}): Promise<T> {
  const isGet = method.toUpperCase() === 'GET';
  const useCache = isGet && !opts.noCache;
  const key = `${method.toUpperCase()} ${url}`;

  if (useCache) {
    // Fast path: TTL hit.
    const hit = sessionCache.get<T>(key);
    if (hit !== undefined) return hit;
    // Dedup path: same key already in flight.
    return sessionCache.getOrCreatePending<T>(key, async () => {
      const data = await fetchJson<T>(url, { ...opts, method });
      sessionCache.set(key, data, opts.pageName);
      return data;
    });
  }

  // Non-GET or noCache: straight to the network, no dedup, no store.
  return fetchJson<T>(url, { ...opts, method });
}

/**
 * Extract the `:pageName` segment from a `/pages/:pageName[/...]` URL.
 * Returns `undefined` if the URL doesn't match the pattern, so callers
 * can silently skip invalidation when the URL is unrelated.
 */
function pageNameFromUrl(url: string): string | undefined {
  const m = url.match(/^\/pages\/([^/?]+)(?:\/|$|\?)/);
  if (!m) return undefined;
  try {
    return decodeURIComponent(m[1]);
  } catch {
    return m[1];
  }
}

/**
 * Drop every cached entry tied to a given page. Called after any
 * mutation that can change the page's content. Safe to call with
 * `undefined` (no-op).
 */
function invalidatePageCache(pageName: string | undefined): void {
  if (!pageName) return;
  sessionCache.invalidatePage(pageName);
}

// ──── Block transformer ─────────────────────────────────────────
// The backend returns `properties` as a `Record<string, unknown>` map.
// The rest of the frontend uses `BlockProperty[]`. Normalize here.

/** Raw block shape as returned by the API (with `properties` as a map). */
interface RawBlock extends Omit<Block, 'properties'> {
  properties?: Record<string, unknown>;
}

function normalizeBlock(raw: RawBlock): Block {
  return {
    ...raw,
    properties: blockPropertiesFromMap(raw.properties),
  } as Block
}

// ──── API ───────────────────────────────────────────────────────

export const api = {
  // Base URL for the API server (empty for same-origin)
  baseUrl: '',
  // ─── Cache control (F of quilt-fase5-session-cache) ───────────
  //
  // Test hook + escape hatch for callers that detect a graph-wide
  // mutation outside the api client (e.g. an SSE event for a
  // different browser tab). Calling this drops every cached GET
  // body; in-flight requests are NOT cancelled — they finish and
  // populate the cache normally.
  invalidateAll: () => sessionCache.invalidateAll(),
  // Pages
  listPages: () =>
    cachedFetch<Page[]>('GET', `/pages`),

  /**
   * Server-side page-name search — S2-03.
   *
   * Replaces the previous pattern of calling `listPages()` and
   * client-side `Array.prototype.includes` filtering on every keystroke,
   * which was O(n) AND transferred the full page list (multi-MB on
   * large graphs) on every search-modal open.
   *
   * The server endpoint is `GET /api/v1/pages/search?q=&limit=` and
   * returns `Page[]`. The optional `limit` defaults to 50 server-side
   * and is clamped to [1, 200]. The endpoint is also safe to call
   * with `q === ''` — the server returns up to `limit` pages
   * ordered by name, so the frontend can drive the "empty query"
   * and "typed query" cases from a single function.
   *
   * The cache key includes the full URL (q + limit) so two distinct
   * queries do not share a cached body. There is no per-page invalidation
   * here because the search result set is a derived view, not the
   * canonical page data — mutations are handled by `createPage` /
   * `updatePage` invalidating the canonical endpoints.
   */
  searchPages: (query: string, limit?: number) => {
    const params = new URLSearchParams()
    params.set('q', query)
    if (limit !== undefined) params.set('limit', String(limit))
    return cachedFetch<Page[]>('GET', `/pages/search?${params.toString()}`)
  },

  getPage: (name: string) =>
    cachedFetch<Page>('GET', `/pages/${encodeURIComponent(name)}`, { pageName: name }),

  createPage: (data: CreatePageRequest) =>
    fetchJson<Page>(`/pages`, { method: 'POST', body: JSON.stringify(data) })
      .then(page => {
        // Creating a page changes the list AND introduces a new
        // page-by-name entry that we want to fetch fresh on next
        // access. Drop both.
        sessionCache.invalidatePage(data.name)
        sessionCache.invalidate('GET /pages')
        return page
      }),

  /**
   * Create a new page by cloning a template's block tree.
   *
   * The template must be a page whose name starts with `template/`
   * (e.g. `template/daily-note`). The server substitutes `{{var}}` /
   * `${var}` placeholders in block content with the new page's name
   * and any user-supplied variables.
   *
   * @see ADR-0003
   */
  createPageFromTemplate: (data: CreatePageFromTemplateRequest) =>
    fetchJson<CreatePageFromTemplateResponse>(`/pages/from-template`, {
      method: 'POST',
      body: JSON.stringify({
        templateName: data.templateName,
        pageName: data.pageName,
        title: data.title,
        variables: data.variables,
      }),
    }).then(page => {
      // New page means a new cache entry should be fetched, and the
      // list is now stale.
      sessionCache.invalidatePage(data.pageName)
      sessionCache.invalidate('GET /pages')
      return page
    }),

  getPageBlocks: async (name: string): Promise<Block[]> => {
    const raw = await cachedFetch<RawBlock[]>(
      'GET',
      `/pages/${encodeURIComponent(name)}/blocks`,
      { pageName: name },
    )
    return raw.map(normalizeBlock)
  },

  getJournal: (date: string) =>
    cachedFetch<Page>('GET', `/pages/journal/${date}`),

  // Backlinks
  getPageBacklinks: (name: string) =>
    cachedFetch<Backlink[]>('GET', `/pages/${encodeURIComponent(name)}/backlinks`, { pageName: name }),

  /**
   * Set or clear the user-edited context override for a single
   * backlink. Q028 (Editable Backlinks).
   *
   * The `:blockId` is the UUID of the **source** block — the block
   * whose content contains the `[[target]]` link. The target is
   * identified by the current `targetPageName` (the page being
   * viewed in the Backlinks panel).
   *
   * Pass `context: null` to clear the override; the panel will fall
   * back to the source block's content snippet. Pass an empty
   * string to explicitly blank out the snippet (same effect as
   * `null` from the server's perspective — the GET endpoint will
   * fall back to the default).
   *
   * The endpoint is intentionally NOT a `PUT` on
   * `/pages/:name/backlinks` because a single source block can have
   * multiple outgoing references — the URL must address the
   * specific `(source, target)` pair.
   */
  updateReferenceContext: (params: {
    sourceBlockId: string;
    targetPageName: string;
    context: string | null;
  }): Promise<Backlink> =>
    fetchJson<Backlink>(
      `/references/${encodeURIComponent(params.sourceBlockId)}?targetPage=${encodeURIComponent(params.targetPageName)}`,
      {
        method: 'PUT',
        body: JSON.stringify({ context: params.context }),
      },
    ).then(dto => {
      // The backlinks list for the target page is now stale — drop it
      // so the next read refetches. Use the same page-key the GET
      // uses so we don't have to know the cache key format.
      sessionCache.invalidatePage(params.targetPageName)
      return dto
    }),

  // Blocks
  createBlock: async (data: CreateBlockRequest): Promise<Block> => {
    const raw = await fetchJson<RawBlock>(`/blocks`, {
      method: 'POST',
      body: JSON.stringify(data),
    })
    // The block list for the parent page is now stale. Drop both the
    // page-by-name and page-blocks entries so the next read refetches.
    invalidatePageCache(data.pageName)
    return normalizeBlock(raw)
  },

  updateBlock: async (id: string, data: UpdateBlockRequest, pageName?: string): Promise<Block> => {
    const raw = await fetchJson<RawBlock>(`/blocks/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
    // If the caller tells us which page the block belongs to, drop
    // that page's cache so the next read shows the updated content.
    // Without `pageName` we can't be precise — the 30s TTL bounds
    // the staleness window, which is the safety net.
    invalidatePageCache(pageName)
    return normalizeBlock(raw)
  },

  deleteBlock: (id: string, pageName?: string) =>
    fetchJson<{ deleted: true }>(`/blocks/${id}`, { method: 'DELETE' })
      .then(result => {
        invalidatePageCache(pageName)
        return result
      }),

  // Block search
  //
  // The backend exposes two equivalent search endpoints:
  //   - GET /api/v1/blocks/search?query=...  (returns `SearchResultDto[]`)
  //   - GET /api/v1/search?q=...             (same shape)
  //
  // The `?query=` name is what the `SearchBlocksParams` struct on the
  // server expects (`crates/quilt-server/src/handlers/blocks.rs:114-120`).
  // Note: this is *not* a `Block[]` — the Rust DTO is `SearchResultDto`
  // with `blockId` / `pageName` / `content` / `snippet` / `score` fields.
  // G3 of the wikilinks audit wires the search modal here so users can
  // find blocks by content.
  searchBlocks: async (query: string, limit = 8): Promise<SearchResult[]> => {
    return cachedFetch<SearchResult[]>(
      'GET',
      `/blocks/search?query=${encodeURIComponent(query)}&limit=${limit}`,
    )
  },

  /**
   * List blocks created by a specific author (e.g. `agent::claude`,
   * `user::alice`). Powers the `/created-by` filter and the agent
   * activity panel. ADR-0003.
   */
  listBlocksByAuthor: async (author: string, limit = 50): Promise<Block[]> => {
    const raw = await cachedFetch<RawBlock[]>(
      'GET',
      `/blocks/by-author?author=${encodeURIComponent(author)}&limit=${limit}`,
    )
    return raw.map(normalizeBlock)
  },

  /**
   * Get the distinct set of agent identifiers that have ever authored
   * a block, sorted ASC. The server filters the result to values
   * starting with `agent::` (so `user::alice` is excluded).
   *
   * Powers the `AgentActivityFeed` panel. Replacing the previous
   * hardcoded `['agent::claude', 'agent::gemini', 'agent::gpt',
   * 'agent::quilt']` list — S2-02.
   *
   * Returns an empty array on a cold graph (no agent authors yet).
   */
  getDistinctAuthors: () =>
    cachedFetch<string[]>('GET', '/blocks/authors'),

  // Settings
  getSettings: () =>
    cachedFetch<UserSettings>('GET', `/settings`),

  updateSettings: (data: UpdateSettingsRequest) =>
    fetchJson<UserSettings>(`/settings`, { method: 'PUT', body: JSON.stringify(data) })
      .then(settings => {
        // Settings affect almost every rendered page (timezone,
        // date format, etc.) — drop everything rather than risk
        // a stale view that doesn't reflect the user's last save.
        sessionCache.invalidateAll()
        return settings
      }),

  getDateFormats: () =>
    cachedFetch<DateFormatOption[]>('GET', `/settings/formats`),

  // Block Properties
  getBlockProperties: (blockId: string) =>
    cachedFetch<BlockProperty[]>('GET', `/blocks/${blockId}/properties`),

  setBlockProperty: (blockId: string, key: string, value: unknown) =>
    fetchJson<void>(`/blocks/${blockId}/properties`, {
      method: 'PUT',
      body: JSON.stringify({ key, value }),
    }),

  deleteBlockProperty: (blockId: string, key: string) =>
    fetchJson<void>(`/blocks/${blockId}/properties/${encodeURIComponent(key)}`, {
      method: 'DELETE',
    }),

  /**
   * List distinct top-level property keys that appear in any block's
   * `properties` JSON column, paginated by key (lexicographic ASC,
   * forward-only, key-as-cursor).
   *
   * Powers the kanban board's "Group by" dropdown and the table view's
   * filter-chip dropdown — both of which used to call
   * `getBlockProperties('')` (an empty block ID, which 404s). The
   * endpoint is mounted at `/api/v1/properties/keys?cursor=&limit=`
   * in `crates/quilt-server/src/routes.rs:40`.
   *
   * @param cursor  Optional cursor — keys are strictly greater than this.
   *                The server rejects the empty string as a 400; pass
   *                `undefined` for the first page.
   * @param limit   Optional page size. Server bounds: 1..=100 (default 50).
   */
  listPropertyKeys: (cursor?: string, limit?: number) => {
    const params = new URLSearchParams()
    if (cursor !== undefined) params.set('cursor', cursor)
    if (limit !== undefined) params.set('limit', String(limit))
    const qs = params.toString()
    const url = qs ? `/properties/keys?${qs}` : `/properties/keys`
    return fetchJson<{
      keys: string[]
      nextCursor: string | null
    }>(url)
  },

  // Graph Lens V1 (subgraph endpoint)
  //
  // Returns a focused subgraph of the knowledge graph centered on
  // the given focus selector. See the backend handler at
  // `crates/quilt-server/src/handlers/graph.rs` for the focus
  // grammar (`block:<uuid>`, `page:<name>`, `property:<key>`) and
  // depth semantics (1..=3, default 1). The "All" lens on the
  // graph view skips this endpoint and uses the page-level
  // `listPages` + `getPageBacklinks` pipeline — call this only
  // when a non-empty `focus` or a specific `depth` is needed.
  getGraphLens: (params: { focus?: string; depth?: number } = {}) => {
    const search = new URLSearchParams()
    if (params.focus) search.set('focus', params.focus)
    if (params.depth !== undefined) search.set('depth', String(params.depth))
    const qs = search.toString()
    return fetchJson<{
      focus: string | null
      depth: number
      nodes: Array<{
        id: string
        content: string
        pageId: string
        pageName: string
        isJournal: boolean
        hasProperties: boolean
      }>
      edges: Array<{ from: string; to: string; kind: 'parent-child' | 'ref' }>
    }>(`/graph/lens${qs ? `?${qs}` : ''}`)
  },

  // Templates (ADR-0007)
  //
  // Lists `template/*` pages with their card metadata (card-shape,
  // icon, cssclass). Powers the EmptyState's template picker so the
  // user can create blocks with `template:: <name>` from a real list
  // of available templates.
  listTemplates: () =>
    cachedFetch<TemplateSummary[]>('GET', `/templates`),

  getTemplateSchema: (name: string) =>
    cachedFetch<TemplateSchema>('GET', `/templates/${encodeURIComponent(name)}/schema`),

  // ─── TODO: Unmounted endpoints removed in P0 fix ───────────────────
  //
  // The following methods used to live here but were removed because
  // their target routes are NOT registered in
  // `crates/quilt-server/src/routes.rs`. Calling them caused runtime
  // 404s / unhandled promise rejections. Re-add them only after the
  // matching server route is mounted:
  //
  //   - `getSchemaPack(name)`        →  GET /api/v1/templates/:name/schema-pack
  //   - `getAnalysisMirror()`         →  GET /api/v1/analysis/mirror
  //   - `getAnalysisConnections(n)`   →  GET /api/v1/analysis/connections
  //   - `getAnalysisGardener()`       →  GET /api/v1/analysis/gardener
  //
  // The `api surface (P0 — only mounted routes)` test in
  // `src/core/__tests__/api-client.test.ts` will fail until each
  // method is both re-added AND its route is registered on the
  // server. That is the contract that prevents the P0 from
  // regressing.
  //
  // The DTOs that used to back these methods (MirrorAnalysisDto,
  // ConnectionDto, GardenerDto, etc.) have been moved to a TODO
  // block at the bottom of this file to be revived alongside them.

  // Query execution (F18)
  executeQuery: async (
    ast: QueryAst,
    limit = 100,
    signal?: AbortSignal,
  ): Promise<QueryResult> => {
    // Enforce limit bounds server-side
    const effectiveLimit = Math.min(Math.max(1, limit), 1000);

    let lastError: Error | null = null;

    // We use fetchJson but with a signal for cancellation
    const authHeaders: Record<string, string> = {};
    if (API_KEY) {
      authHeaders['Authorization'] = `Bearer ${API_KEY}`;
    }

    const res = await fetch(`${API_BASE}/query`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...authHeaders,
      },
      body: JSON.stringify({ ast, limit: effectiveLimit }),
      signal,
    });

    if (!res.ok) {
      let code = 'SERVER_ERROR';
      let detail = res.statusText;
      try {
        const body = await res.json();
        code = body.code || code;
        detail = body.error || detail;
      } catch {
        // ignore parse error
      }

      if (res.status === 401) {
        const err: QueryError = { type: 'Unauthorized', message: detail };
        throw err;
      }
      if (res.status === 422) {
        const err: QueryError = { type: 'InvalidAst', message: detail };
        throw err;
      }
      if (res.status === 413) {
        const err: QueryError = { type: 'InvalidAst', message: 'Query too large (>64KB)' };
        throw err;
      }
      const err: QueryError = { type: 'ServerError', message: detail };
      throw err;
    }

    return res.json() as Promise<QueryResult>;
  },

  // ─── Tour state (B of quilt-fase4-cross-device-tour) ────────────────
  //
  // Server-stored dismissal state for first-run product tours. The
  // localStorage flag remains a fast-render cache; the server is the
  // source of truth so a dismissal on desktop also hides the tour on
  // mobile. The api key (Authorization: Bearer) is the user
  // identifier for V1.
  //
  // Names are short slugs ("welcome", "cognitive", "mcp") — the same
  // shape used by the SQLite `tour_dismissals` table on the server.

  /**
   * GET /api/v1/user/tour-state
   * Returns the alphabetically-sorted list of tour names the current
   * user has dismissed. Empty array for a first-time visitor.
   */
  getTourState: () => cachedFetch<TourStateResponse>('GET', '/user/tour-state'),

  /**
   * POST /api/v1/user/tour-state/dismiss
   * Marks a tour as dismissed for the current user. Idempotent —
   * the server returns the updated dismissed-list so the caller
   * doesn't have to re-fetch.
   *
   * Throws `QuiltApiError(400)` if the tour name is empty, too long,
   * or contains whitespace / control characters.
   */
  dismissTour: (tourName: string) =>
    fetchJson<TourStateResponse>('/user/tour-state/dismiss', {
      method: 'POST',
      body: JSON.stringify({ tour: tourName } satisfies DismissTourRequest),
    }).then(result => {
      // The dismissed list just changed — drop the cached GET so
      // the next caller (or the optimistic localStorage merge on
      // hydration) sees the new state from the server.
      sessionCache.invalidate('GET /user/tour-state')
      return result
    }),

  // ─── Projection (ADR-0025) ─────────────────────────────────────────────────
  //
  // `GET /api/v1/blocks/:id/projection` — resolves the winning projection
  // for a block and returns the `ProjectionView`. The cache TTL is 30s
  // to balance freshness with responsiveness.

  /**
   * Get the resolved projection view for a block.
   *
   * The projection aggregates the block's visual surface: text content,
   * decorations (badges, checkboxes, etc.), links, and any resolution
   * conflicts. Powers the projection-renderer feature.
   *
   * @param blockId UUID of the block to project
   */
  getBlockProjection: (blockId: string) =>
    cachedFetch<import('@features/projection/types').ProjectionView>(
      'GET',
      `/blocks/${blockId}/projection`,
    ),

  // ─── Property presets (ADR-0025) ──────────────────────────────────────────
  //
  // `GET /api/v1/presets` — lists all registered V1 property presets.
  // Each preset is a named bundle of property patches (key/value/mutability)
  // that can be applied to a block to change its visual representation.

  /**
   * List all available property presets.
   *
   * Presets power the slash command menu and the preset palette. Each
   * preset carries required argument kinds (date/url/text) that the
   * caller must provide when applying.
   */
  listPresets: () =>
    cachedFetch<import('@features/projection/types').PresetListResponse>(
      'GET',
      '/presets',
    ),
};

// ─── TODO: Analysis DTOs (G7 Dream Cycle) ───────────────────────────────
//
// The DTOs that used to back `getAnalysisMirror`, `getAnalysisConnections`,
// and `getAnalysisGardener` are preserved here so the schema isn't lost
// when those routes are re-mounted. They are NOT exported — consumers
// (MirrorPanel, SerendipityFeed) were also removed and should be
// recreated fresh from this contract.
//
// To re-enable:
//   1. Mount the routes in `crates/quilt-server/src/routes.rs`:
//        .nest("/api/v1/analysis", handlers::cognitive::analysis_routes())
//      (the handler module `cognitive.rs` already defines them)
//   2. Re-export the DTOs (remove the `TODO` comment) and re-add the
//      three `getAnalysis*` methods on the `api` object above.
//   3. Recreate the consumer components under
//      `src/features/cognitive/` and import from this module.

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoMirrorAnalysisDto {
  clusters: _TodoClusterDto[]
  gaps: _TodoGapDto[]
  frontiers: string[]
  density: number
  top_influencers: _TodoInfluencerDto[]
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoClusterDto {
  block_ids: string[]
  theme: string | null
  coherence_score: number
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoGapDto {
  from_block: string
  to_block: string
  shared_refs: string[]
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoInfluencerDto {
  block_id: string
  influence_score: number
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoConnectionDto {
  pairs: _TodoConnectionPairDto[]
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoConnectionPairDto {
  block_a: string
  block_b: string
  score: number
  reason: string
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoGardenerDto {
  beliefs: _TodoBeliefDto[]
  suggestions: _TodoDeepeningSuggestionDto[]
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoBeliefDto {
  id: string
  statement: string
  confidence: number
  last_updated: string
}

/** @internal — TODO: re-export when analysis routes are mounted */
interface _TodoDeepeningSuggestionDto {
  concept: string
  current_depth: number
  suggested_questions: string[]
}
