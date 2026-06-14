/**
 * usePage — focused hook for page data fetching.
 *
 * Composable pattern: each hook does one thing well. Page data concerns
 * (loading, error, fetched page/list) are isolated here so callers can
 * compose with useBlocks, useNavigation, etc.
 */
import { useState, useEffect, useCallback } from 'react'
import { api } from '@core/api-client'
import type { Page, CreatePageRequest } from '@shared/types/api'

interface UsePageOptions {
  /** Page name to fetch. If omitted, fetches the page list. */
  pageName?: string
  /** Skip the fetch entirely when true. */
  enabled?: boolean
}

interface UsePageResult {
  /** The fetched page (single-page mode). */
  page: Page | null
  /** All pages (list mode). */
  pages: Page[]
  /** Currently loading. */
  loading: boolean
  /** Error message if the last fetch failed. */
  error: string | null
  /** Refresh the data (refetches from API, bypasses cache). */
  refresh: () => void
  /** Create a new page and optionally navigate to it. */
  createPage: (data: CreatePageRequest) => Promise<Page | null>
}

/**
 * Single-page mode: fetches one page by name.
 * Set `pageName` in options to activate.
 */
export function usePage(options: UsePageOptions): UsePageResult {
  const { pageName, enabled = true } = options
  const [page, setPage] = useState<Page | null>(null)
  const [pages, setPages] = useState<Page[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchPage = useCallback(async (bypassCache = false) => {
    if (!pageName || !enabled) return
    setLoading(true)
    setError(null)
    try {
      const data = await api.getPage(pageName)
      setPage(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load page')
      setPage(null)
    } finally {
      setLoading(false)
    }
  }, [pageName, enabled])

  // Refetch when pageName changes
  useEffect(() => {
    if (!pageName || !enabled) return
    fetchPage()
  }, [pageName, enabled, fetchPage])

  const refresh = useCallback(() => {
    fetchPage(true)
  }, [fetchPage])

  const createPage = useCallback(async (data: CreatePageRequest): Promise<Page | null> => {
    try {
      const newPage = await api.createPage(data)
      setPages(prev => [...prev, newPage])
      return newPage
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create page')
      return null
    }
  }, [])

  return { page, pages, loading, error, refresh, createPage }
}

/**
 * List-mode: fetches all pages (non-journal).
 * Activated when `pageName` is omitted from options.
 */
export function usePageList(options: Omit<UsePageOptions, 'pageName'> = {}): Omit<UsePageResult, 'page' | 'createPage'> & {
  pages: Page[]
  searchPages: (query: string, limit?: number) => Promise<Page[]>
} {
  const { enabled = true } = options
  const [pages, setPages] = useState<Page[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchPages = useCallback(async (bypassCache = false) => {
    if (!enabled) return
    setLoading(true)
    setError(null)
    try {
      const data = await api.listPages()
      setPages(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pages')
    } finally {
      setLoading(false)
    }
  }, [enabled])

  useEffect(() => {
    fetchPages()
  }, [fetchPages])

  const refresh = useCallback(() => {
    fetchPages(true)
  }, [fetchPages])

  const searchPages = useCallback(async (query: string, limit?: number): Promise<Page[]> => {
    return api.searchPages(query, limit)
  }, [])

  return { pages, loading, error, refresh, searchPages }
}
