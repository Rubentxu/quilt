import type {
  Annotation,
  AnnotationFilters,
  CreateAnnotationRequest,
  UpdateAnnotationStatusRequest,
} from '@shared/types/api'
import { fetchJson } from './client'

export const annotationsApi = {
  createAnnotation: (data: CreateAnnotationRequest) =>
    fetchJson<Annotation>('/annotations', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  listAnnotations: (filters: AnnotationFilters = {}) => {
    const params = new URLSearchParams()
    if (filters.blockId) params.set('block_id', filters.blockId)
    if (filters.status) params.set('status', filters.status)
    if (filters.scope) params.set('scope', filters.scope)
    if (filters.authorName) params.set('author_name', filters.authorName)
    const qs = params.toString()
    return fetchJson<Annotation[]>(`/annotations${qs ? `?${qs}` : ''}`)
  },

  listAnnotationsForBlock: (blockId: string) =>
    fetchJson<Annotation[]>(`/blocks/${encodeURIComponent(blockId)}/annotations`),

  getAnnotation: (id: string) =>
    fetchJson<Annotation>(`/annotations/${encodeURIComponent(id)}`),

  updateAnnotationStatus: (id: string, data: UpdateAnnotationStatusRequest) =>
    fetchJson<Annotation>(`/annotations/${encodeURIComponent(id)}/status`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    }),

  resolveAnnotation: (id: string, resolvedBy: string) =>
    fetchJson<Annotation>(`/annotations/${encodeURIComponent(id)}/status`, {
      method: 'PATCH',
      body: JSON.stringify({ status: 'resolved', resolvedBy } satisfies UpdateAnnotationStatusRequest),
    }),

  deleteAnnotation: (id: string) =>
    fetchJson<void>(`/annotations/${encodeURIComponent(id)}`, { method: 'DELETE' }),
}
