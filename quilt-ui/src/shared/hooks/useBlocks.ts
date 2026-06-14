/**
 * useBlocks — focused hook for block operations.
 *
 * Composable pattern: block CRUD and list operations are isolated here.
 * Composes with usePage to get the current pageName.
 */
import { useState, useEffect, useCallback, useRef } from 'react'
import { api } from '@core/api-client'
import type { Block, CreateBlockRequest, UpdateBlockRequest } from '@shared/types/api'

interface UseBlocksOptions {
  /** Page name to fetch blocks for. */
  pageName: string | null
  /** Skip the fetch when true or pageName is null. */
  enabled?: boolean
  /** Callback when blocks change (e.g., after create/update/delete). */
  onBlocksChanged?: (blocks: Block[]) => void
}

interface UseBlocksResult {
  /** Current blocks for the page. */
  blocks: Block[]
  /** Currently loading. */
  loading: boolean
  /** Error message if the last operation failed. */
  error: string | null
  /** Refresh blocks from API (bypasses cache). */
  refresh: () => void
  /** Create a new block. */
  createBlock: (data: CreateBlockRequest) => Promise<Block | null>
  /** Update an existing block. */
  updateBlock: (id: string, data: UpdateBlockRequest) => Promise<Block | null>
  /** Delete a block. */
  deleteBlock: (id: string) => Promise<boolean>
}

/**
 * Fetch and manage blocks for a single page.
 * The `pageName` is the re-init key — changing it re-fetches.
 */
export function useBlocks({
  pageName,
  enabled = true,
  onBlocksChanged,
}: UseBlocksOptions): UseBlocksResult {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // Keep callback ref fresh without re-triggering the effect
  const onBlocksChangedRef = useRef(onBlocksChanged)
  onBlocksChangedRef.current = onBlocksChanged

  const fetchBlocks = useCallback(async (_bypassCache = false) => {
    if (!pageName || !enabled) return
    setLoading(true)
    setError(null)
    try {
      const data = await api.getPageBlocks(pageName)
      setBlocks(data)
      onBlocksChangedRef.current?.(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load blocks')
    } finally {
      setLoading(false)
    }
  }, [pageName, enabled])

  // Re-fetch when pageName changes
  useEffect(() => {
    if (!pageName || !enabled) return
    fetchBlocks()
  }, [pageName, enabled, fetchBlocks])

  const refresh = useCallback(() => {
    fetchBlocks(true)
  }, [fetchBlocks])

  const createBlock = useCallback(async (data: CreateBlockRequest): Promise<Block | null> => {
    try {
      const newBlock = await api.createBlock(data)
      // Optimistically add to the list if pageName matches
      if (data.pageName === pageName) {
        setBlocks(prev => [...prev, newBlock])
        onBlocksChangedRef.current?.([...blocks, newBlock])
      }
      return newBlock
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create block')
      return null
    }
  }, [pageName, blocks])

  const updateBlock = useCallback(async (id: string, data: UpdateBlockRequest): Promise<Block | null> => {
    try {
      const updated = await api.updateBlock(id, data, pageName ?? undefined)
      setBlocks(prev => prev.map(b => b.id === id ? updated : b))
      onBlocksChangedRef.current?.(blocks.map(b => b.id === id ? updated : b))
      return updated
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update block')
      return null
    }
  }, [pageName, blocks])

  const deleteBlock = useCallback(async (id: string): Promise<boolean> => {
    try {
      await api.deleteBlock(id, pageName ?? undefined)
      const newBlocks = blocks.filter(b => b.id !== id)
      setBlocks(newBlocks)
      onBlocksChangedRef.current?.(newBlocks)
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete block')
      return false
    }
  }, [pageName, blocks])

  return { blocks, loading, error, refresh, createBlock, updateBlock, deleteBlock }
}
