/**
 * useAnalysisQuery — G7 Dream Cycle Display
 *
 * Generic hook for fetching analysis data with loading/error states.
 * Provides a consistent interface for all analysis endpoints.
 */

import { useState, useEffect, useCallback, useRef } from 'react'

export interface AnalysisState<T> {
  data: T | null
  loading: boolean
  refreshing: boolean
  error: string | null
  refetch: () => Promise<void>
}

/**
 * Hook for fetching and managing analysis data.
 *
 * @param fetchFn - Async function that fetches the data
 * @param deps - Dependency array that triggers re-fetch when changed
 * @returns AnalysisState with data, loading, error, and refetch function
 */
export function useAnalysisQuery<T>(
  fetchFn: () => Promise<T>,
  deps: unknown[] = [],
): AnalysisState<T> {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchRef = useRef(fetchFn)
  fetchRef.current = fetchFn

  const load = useCallback(
    async (isRefresh: boolean) => {
      if (isRefresh) setRefreshing(true)
      else setLoading(true)
      setError(null)

      try {
        const result = await fetchRef.current()
        setData(result)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load')
      } finally {
        setLoading(false)
        setRefreshing(false)
      }
    },
    [],
  )

  // Initial load
  useEffect(() => {
    void load(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)

  const refetch = useCallback(async () => {
    await load(true)
  }, [load])

  return {
    data,
    loading,
    refreshing,
    error,
    refetch,
  }
}
