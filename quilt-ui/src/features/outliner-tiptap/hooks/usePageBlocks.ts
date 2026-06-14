import { useCallback, useEffect, useRef, useState } from 'react'
import { api } from '@core/api-client'
import { ensureWasmLoaded } from '@core/wasm-bridge/WasmProvider'
import type { Block } from '@shared/types/api'

interface UsePageBlocksArgs {
  pageName: string
  isJournal?: boolean
  wasmLoaded: boolean
  wasmLoadPage: (pageName: string, blocks: Block[]) => void
  onEmptyJournal?: () => void
}

export function usePageBlocks({ pageName, isJournal, wasmLoaded, wasmLoadPage, onEmptyJournal }: UsePageBlocksArgs) {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const blocksRef = useRef<Block[]>(blocks)
  const autoCreatedRef = useRef<Set<string>>(new Set())
  const onEmptyJournalRef = useRef(onEmptyJournal)
  onEmptyJournalRef.current = onEmptyJournal
  blocksRef.current = blocks

  const refreshBlocks = useCallback(async () => {
    const fetchedBlocks = await api.getPageBlocks(pageName)
    setBlocks(fetchedBlocks)
    return fetchedBlocks
  }, [pageName])

  useEffect(() => {
    let cancelled = false

    async function load() {
      setLoading(true)
      setError(null)

      try {
        const fetchedBlocks = await api.getPageBlocks(pageName)
        if (cancelled) return

        if (!wasmLoaded) {
          try {
            await ensureWasmLoaded()
          } catch {
            if (cancelled) return
            setBlocks(fetchedBlocks)
            setLoading(false)
            return
          }
        }
        if (cancelled) return

        try {
          wasmLoadPage(pageName, fetchedBlocks)
        } catch (error) {
          console.warn('WASM load failed, rendering from API data:', error)
        }

        setBlocks(fetchedBlocks)
        setLoading(false)

        if (isJournal && fetchedBlocks.length === 0 && !autoCreatedRef.current.has(pageName)) {
          autoCreatedRef.current.add(pageName)
          requestAnimationFrame(() => {
            if (cancelled) return
            onEmptyJournalRef.current?.()
          })
        }
      } catch (error) {
        if (!cancelled) {
          setError(error instanceof Error ? error.message : 'Unknown error')
          setLoading(false)
        }
      }
    }

    load()
    return () => {
      cancelled = true
    }
  }, [isJournal, pageName, wasmLoaded, wasmLoadPage])

  return {
    blocks,
    setBlocks,
    blocksRef,
    loading,
    error,
    refreshBlocks,
  }
}
