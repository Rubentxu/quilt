import { useEffect, type Dispatch, type SetStateAction } from 'react'
import type { Block } from '@shared/types/api'
import { getEventsUrl } from '@core/api-client'
import { useConnection } from '@shared/contexts/ConnectionContext'
import { usePollingSync } from '@shared/hooks/usePollingSync'
import { useSSE } from '@shared/hooks/useSSE'

interface UseBlockSyncArgs {
  pageName: string
  setBlocks: Dispatch<SetStateAction<Block[]>>
}

export function useBlockSync({ pageName, setBlocks }: UseBlockSyncArgs) {
  const { connected: sseConnected } = useSSE({
    url: getEventsUrl(),
    onEvent: (event) => {
      switch (event.type) {
        case 'block_updated':
          setBlocks(prev => prev.map(block => (block.id === event.data.id ? { ...block, ...event.data } : block)))
          break
        case 'block_created':
          setBlocks(prev => (prev.some(block => block.id === event.data.id) ? prev : [...prev, event.data]))
          break
        case 'block_deleted':
          setBlocks(prev => prev.filter(block => block.id !== event.data.id))
          break
        case 'page_updated':
          break
      }
    },
    enabled: false,
  })

  const { setSseConnected } = useConnection()

  useEffect(() => {
    setSseConnected(sseConnected)
  }, [sseConnected, setSseConnected])

  usePollingSync({
    pageName,
    interval: 15000,
    onBlocksChanged: setBlocks,
    enabled: !!pageName && !sseConnected,
  })

  return { sseConnected }
}
