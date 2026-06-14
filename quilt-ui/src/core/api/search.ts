import type { Page, SearchResult } from '@shared/types/api'
import type { QueryAst, QueryError, QueryResult } from '@shared/types/queryAst'
import { apiBaseUrl, cachedFetch, createAuthHeaders } from './client'

export const searchApi = {
  searchPages: (query: string, limit?: number) => {
    const params = new URLSearchParams()
    params.set('q', query)
    if (limit !== undefined) params.set('limit', String(limit))
    return cachedFetch<Page[]>('GET', `/pages/search?${params.toString()}`)
  },

  searchBlocks: (query: string, limit = 8): Promise<SearchResult[]> =>
    cachedFetch<SearchResult[]>(
      'GET',
      `/blocks/search?query=${encodeURIComponent(query)}&limit=${limit}`,
    ),

  executeQuery: async (ast: QueryAst, limit = 100, signal?: AbortSignal): Promise<QueryResult> => {
    const effectiveLimit = Math.min(Math.max(1, limit), 1000)
    const res = await fetch(`${apiBaseUrl()}/query`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...createAuthHeaders(),
      },
      body: JSON.stringify({ ast, limit: effectiveLimit }),
      signal,
    })

    if (!res.ok) {
      let detail = res.statusText
      try {
        const body = await res.json()
        detail = body.error || detail
      } catch {
        // ignore parse error
      }

      if (res.status === 401) throw { type: 'Unauthorized', message: detail } satisfies QueryError
      if (res.status === 422) throw { type: 'InvalidAst', message: detail } satisfies QueryError
      if (res.status === 413) throw { type: 'InvalidAst', message: 'Query too large (>64KB)' } satisfies QueryError
      throw { type: 'ServerError', message: detail } satisfies QueryError
    }

    return res.json() as Promise<QueryResult>
  },
}
