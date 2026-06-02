import { useEffect, useRef, useCallback, useState } from 'react'

interface SSEEvent {
  type: string
  data: any
}

interface UseSSEOptions {
  url: string
  onEvent: (event: SSEEvent) => void
  enabled?: boolean
  reconnectInterval?: number
}

export function useSSE({ url, onEvent, enabled = true, reconnectInterval = 5000 }: UseSSEOptions) {
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const eventSourceRef = useRef<EventSource | null>(null)
  const onEventRef = useRef(onEvent)
  onEventRef.current = onEvent

  const connect = useCallback(() => {
    if (!enabled) return

    // Close existing connection if any
    eventSourceRef.current?.close()

    try {
      const es = new EventSource(url)

      es.onopen = () => {
        setConnected(true)
        setError(null)
      }

      es.onmessage = (e: MessageEvent) => {
        try {
          const parsed = JSON.parse(e.data) as SSEEvent
          onEventRef.current(parsed)
        } catch {
          // Non-JSON message, ignore
        }
      }

      // Listen for specific event types
      es.addEventListener('block_updated', (e: Event) => {
        try {
          const data = JSON.parse((e as MessageEvent).data)
          onEventRef.current({ type: 'block_updated', data })
        } catch { /* ignore */ }
      })

      es.addEventListener('block_created', (e: Event) => {
        try {
          const data = JSON.parse((e as MessageEvent).data)
          onEventRef.current({ type: 'block_created', data })
        } catch { /* ignore */ }
      })

      es.addEventListener('block_deleted', (e: Event) => {
        try {
          const data = JSON.parse((e as MessageEvent).data)
          onEventRef.current({ type: 'block_deleted', data })
        } catch { /* ignore */ }
      })

      es.addEventListener('page_updated', (e: Event) => {
        try {
          const data = JSON.parse((e as MessageEvent).data)
          onEventRef.current({ type: 'page_updated', data })
        } catch { /* ignore */ }
      })

      es.onerror = () => {
        setConnected(false)
        setError('SSE connection lost')
        es.close()
        // Auto-reconnect
        setTimeout(() => {
          if (enabled) connect()
        }, reconnectInterval)
      }

      eventSourceRef.current = es
    } catch (err) {
      setError(err instanceof Error ? err.message : 'SSE connection failed')
    }
  }, [url, enabled, reconnectInterval])

  useEffect(() => {
    connect()
    return () => {
      eventSourceRef.current?.close()
      eventSourceRef.current = null
    }
  }, [connect])

  const disconnect = useCallback(() => {
    eventSourceRef.current?.close()
    eventSourceRef.current = null
    setConnected(false)
  }, [])

  return { connected, error, disconnect, reconnect: connect }
}
