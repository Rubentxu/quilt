import type {
  Block,
  BlockProperty,
  CreateBlockRequest,
  SearchResult,
  UpdateBlockRequest,
} from '@shared/types/api'
import { cachedFetch, fetchJson, invalidatePageCache, normalizeBlock, type RawBlock } from './client'

export const blocksApi = {
  getPageBlocks: async (name: string): Promise<Block[]> => {
    const raw = await cachedFetch<RawBlock[]>('GET', `/pages/${encodeURIComponent(name)}/blocks`, { pageName: name })
    return raw.map(normalizeBlock)
  },

  createBlock: async (data: CreateBlockRequest): Promise<Block> => {
    const raw = await fetchJson<RawBlock>('/blocks', {
      method: 'POST',
      body: JSON.stringify(data),
    })
    invalidatePageCache(data.pageName)
    return normalizeBlock(raw)
  },

  updateBlock: async (id: string, data: UpdateBlockRequest, pageName?: string): Promise<Block> => {
    const raw = await fetchJson<RawBlock>(`/blocks/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
    invalidatePageCache(pageName)
    return normalizeBlock(raw)
  },

  deleteBlock: (id: string, pageName?: string) =>
    fetchJson<{ deleted: true }>(`/blocks/${id}`, { method: 'DELETE' }).then(result => {
      invalidatePageCache(pageName)
      return result
    }),

  listBlocksByAuthor: async (author: string, limit = 50): Promise<Block[]> => {
    const raw = await cachedFetch<RawBlock[]>(
      'GET',
      `/blocks/by-author?author=${encodeURIComponent(author)}&limit=${limit}`,
    )
    return raw.map(normalizeBlock)
  },

  getDistinctAuthors: () => cachedFetch<string[]>('GET', '/blocks/authors'),

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

  listPropertyKeys: (cursor?: string, limit?: number) => {
    const params = new URLSearchParams()
    if (cursor !== undefined) params.set('cursor', cursor)
    if (limit !== undefined) params.set('limit', String(limit))
    const qs = params.toString()
    const url = qs ? `/properties/keys?${qs}` : '/properties/keys'
    return fetchJson<{
      keys: string[]
      nextCursor: string | null
    }>(url)
  },
}
