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
} from '@shared/types/api';
import type { QueryAst, QueryError, QueryResult } from '@shared/types/queryAst';
import { blockPropertiesFromMap } from '@shared/utils/blockProperties';

const API_BASE = '/api/v1';

/** Auth token loaded from environment — all API calls include `Authorization: Bearer <token>` */
const API_KEY = import.meta.env.VITE_QUILT_API_KEY || '';

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
  // Pages
  listPages: () =>
    fetchJson<Page[]>(`/pages`),

  getPage: (name: string) =>
    fetchJson<Page>(`/pages/${encodeURIComponent(name)}`),

  createPage: (data: CreatePageRequest) =>
    fetchJson<Page>(`/pages`, { method: 'POST', body: JSON.stringify(data) }),

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
    }),

  getPageBlocks: async (name: string): Promise<Block[]> => {
    const raw = await fetchJson<RawBlock[]>(
      `/pages/${encodeURIComponent(name)}/blocks`,
    )
    return raw.map(normalizeBlock)
  },

  getJournal: (date: string) =>
    fetchJson<Page>(`/pages/journal/${date}`),

  // Backlinks
  getPageBacklinks: (name: string) =>
    fetchJson<Backlink[]>(`/pages/${encodeURIComponent(name)}/backlinks`),

  // Blocks
  createBlock: async (data: CreateBlockRequest): Promise<Block> => {
    const raw = await fetchJson<RawBlock>(`/blocks`, {
      method: 'POST',
      body: JSON.stringify(data),
    })
    return normalizeBlock(raw)
  },

  updateBlock: async (id: string, data: UpdateBlockRequest): Promise<Block> => {
    const raw = await fetchJson<RawBlock>(`/blocks/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
    return normalizeBlock(raw)
  },

  deleteBlock: (id: string) =>
    fetchJson<{ deleted: true }>(`/blocks/${id}`, { method: 'DELETE' }),

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
    return fetchJson<SearchResult[]>(
      `/blocks/search?query=${encodeURIComponent(query)}&limit=${limit}`,
    )
  },

  /**
   * List blocks created by a specific author (e.g. `agent::claude`,
   * `user::alice`). Powers the `/created-by` filter and the agent
   * activity panel. ADR-0003.
   */
  listBlocksByAuthor: async (author: string, limit = 50): Promise<Block[]> => {
    const raw = await fetchJson<RawBlock[]>(
      `/blocks/by-author?author=${encodeURIComponent(author)}&limit=${limit}`,
    )
    return raw.map(normalizeBlock)
  },

  // Settings
  getSettings: () =>
    fetchJson<UserSettings>(`/settings`),

  updateSettings: (data: UpdateSettingsRequest) =>
    fetchJson<UserSettings>(`/settings`, { method: 'PUT', body: JSON.stringify(data) }),

  getDateFormats: () =>
    fetchJson<DateFormatOption[]>(`/settings/formats`),

  // Block Properties
  getBlockProperties: (blockId: string) =>
    fetchJson<BlockProperty[]>(`/blocks/${blockId}/properties`),

  setBlockProperty: (blockId: string, key: string, value: unknown) =>
    fetchJson<void>(`/blocks/${blockId}/properties`, {
      method: 'PUT',
      body: JSON.stringify({ key, value }),
    }),

  deleteBlockProperty: (blockId: string, key: string) =>
    fetchJson<void>(`/blocks/${blockId}/properties/${encodeURIComponent(key)}`, {
      method: 'DELETE',
    }),

  // Templates (ADR-0007)
  //
  // Lists `template/*` pages with their card metadata (card-shape,
  // icon, cssclass). Powers the EmptyState's template picker so the
  // user can create blocks with `template:: <name>` from a real list
  // of available templates.
  listTemplates: () =>
    fetchJson<TemplateSummary[]>(`/templates`),

  getTemplateSchema: (name: string) =>
    fetchJson<TemplateSchema>(`/templates/${encodeURIComponent(name)}/schema`),

  // Schema Pack (G6)
  getSchemaPack: (name: string) =>
    fetchJson<{ schema_pack: unknown }>(
      `/templates/${encodeURIComponent(name)}/schema-pack`,
    ),

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

  // ─── Analysis endpoints (G7 Dream Cycle) ────────────────────────────

  /**
   * GET /api/v1/analysis/mirror
   * Returns structural mirror analysis: clusters, gaps, frontiers, density.
   */
  getAnalysisMirror: () =>
    fetchJson<MirrorAnalysisDto>('/analysis/mirror'),

  /**
   * GET /api/v1/analysis/connections
   * Returns serendipitous connections between blocks.
   * @param limit Maximum number of connections (clamped to 50 server-side)
   */
  getAnalysisConnections: (limit = 10) =>
    fetchJson<ConnectionDto>(`/analysis/connections?limit=${limit}`),

  /**
   * GET /api/v1/analysis/gardener
   * Returns belief suggestions from the structure gardener.
   */
  getAnalysisGardener: () =>
    fetchJson<GardenerDto>('/analysis/gardener'),
};

// ─── Analysis DTOs ──────────────────────────────────────────────────────

export interface MirrorAnalysisDto {
  clusters: ClusterDto[]
  gaps: GapDto[]
  frontiers: string[]
  density: number
}

export interface ClusterDto {
  block_ids: string[]
  theme: string | null
  coherence_score: number
}

export interface GapDto {
  from_block: string
  to_block: string
  shared_refs: string[]
}

export interface ConnectionDto {
  pairs: ConnectionPairDto[]
}

export interface ConnectionPairDto {
  block_a: string
  block_b: string
  score: number
  reason: string
}

export interface GardenerDto {
  beliefs: BeliefDto[]
  suggestions: DeepeningSuggestionDto[]
}

export interface BeliefDto {
  id: string
  statement: string
  confidence: number
  last_updated: string
}

export interface DeepeningSuggestionDto {
  concept: string
  current_depth: number
  suggested_questions: string[]
}
