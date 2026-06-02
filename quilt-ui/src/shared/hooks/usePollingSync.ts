import { useEffect, useRef } from 'react'
import { api } from '@core/api-client'
import type { Block } from '@shared/types/api'

interface UsePollingSyncOptions {
  pageName: string | null
  interval?: number
  onBlocksChanged: (blocks: Block[]) => void
  enabled?: boolean
}

export function usePollingSync({
  pageName,
  interval = 30000, // 30s default
  onBlocksChanged,
  enabled = true,
}: UsePollingSyncOptions) {
  const timerRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined)
  const onBlocksChangedRef = useRef(onBlocksChanged)
  onBlocksChangedRef.current = onBlocksChanged

  useEffect(() => {
    if (!enabled || !pageName) return

    timerRef.current = setInterval(async () => {
      try {
        const blocks = await api.getPageBlocks(pageName)
        onBlocksChangedRef.current(blocks)
      } catch {
        // Silently fail — don't disrupt the user
      }
    }, interval)

    return () => {
      if (timerRef.current) clearInterval(timerRef.current)
    }
  }, [pageName, interval, enabled])

  return null
}
