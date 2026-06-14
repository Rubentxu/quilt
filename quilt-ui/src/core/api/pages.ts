import type {
  Backlink,
  CreatePageFromTemplateRequest,
  CreatePageFromTemplateResponse,
  CreatePageRequest,
  Page,
  TemplateSchema,
  TemplateSummary,
} from '@shared/types/api'
import { cachedFetch, fetchJson, invalidateCacheKey, invalidatePageCache } from './client'

export const pagesApi = {
  listPages: () => cachedFetch<Page[]>('GET', '/pages'),

  getPage: (name: string) =>
    cachedFetch<Page>('GET', `/pages/${encodeURIComponent(name)}`, { pageName: name }),

  createPage: (data: CreatePageRequest) =>
    fetchJson<Page>('/pages', { method: 'POST', body: JSON.stringify(data) }).then(page => {
      invalidatePageCache(data.name)
      invalidateCacheKey('GET /pages')
      return page
    }),

  createPageFromTemplate: (data: CreatePageFromTemplateRequest) =>
    fetchJson<CreatePageFromTemplateResponse>('/pages/from-template', {
      method: 'POST',
      body: JSON.stringify({
        templateName: data.templateName,
        pageName: data.pageName,
        title: data.title,
        variables: data.variables,
      }),
    }).then(page => {
      invalidatePageCache(data.pageName)
      invalidateCacheKey('GET /pages')
      return page
    }),

  getJournal: (date: string) => cachedFetch<Page>('GET', `/pages/journal/${date}`),

  getPageBacklinks: (name: string) =>
    cachedFetch<Backlink[]>('GET', `/pages/${encodeURIComponent(name)}/backlinks`, { pageName: name }),

  updateReferenceContext: (params: {
    sourceBlockId: string
    targetPageName: string
    context: string | null
  }): Promise<Backlink> =>
    fetchJson<Backlink>(
      `/references/${encodeURIComponent(params.sourceBlockId)}?targetPage=${encodeURIComponent(params.targetPageName)}`,
      {
        method: 'PUT',
        body: JSON.stringify({ context: params.context }),
      },
    ).then(dto => {
      invalidatePageCache(params.targetPageName)
      return dto
    }),

  listTemplates: () => cachedFetch<TemplateSummary[]>('GET', '/templates'),

  getTemplateSchema: (name: string) =>
    cachedFetch<TemplateSchema>('GET', `/templates/${encodeURIComponent(name)}/schema`),

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
}
