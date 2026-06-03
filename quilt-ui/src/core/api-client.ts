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
} from '@shared/types/api';
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
};
