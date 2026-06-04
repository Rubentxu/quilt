/**
 * SerendipityFeed — G7 Dream Cycle Display
 *
 * Displays unexpected connections and serendipitous discoveries.
 * Auto-refreshes every 5 minutes.
 */

import { useState, useEffect, useCallback } from 'react'
import { RefreshCw, Loader2, Lightbulb, Link2 } from 'lucide-react'
import { api, type ConnectionDto } from '@core/api-client'
import { useRefreshInterval } from './useRefreshInterval'

interface SerendipityFeedProps {
  /** Maximum number of connections to display. Default 10. */
  limit?: number
  /** Auto-refresh interval in ms. Default 5 minutes (300000). 0 to disable. */
  refreshInterval?: number
}

export function SerendipityFeed({ limit = 10, refreshInterval = 300000 }: SerendipityFeedProps) {
  const [data, setData] = useState<ConnectionDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)

    try {
      const result = await api.getAnalysisConnections(limit)
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [limit])

  // Initial load
  useEffect(() => {
    void load(false)
  }, [load])

  // Auto-refresh
  useRefreshInterval(() => load(true), refreshInterval)

  if (loading) {
    return (
      <div
        data-testid="serendipity-feed"
        style={{
          padding: 'var(--space-3)',
          color: 'var(--color-text-muted)',
          fontSize: '12px',
        }}
      >
        <Loader2
          size={12}
          style={{ verticalAlign: 'middle', marginRight: '4px' }}
        />
        Loading connections…
      </div>
    )
  }

  return (
    <div data-testid="serendipity-feed" style={{ padding: 'var(--space-2)' }}>
      {/* Header */}
      <div
        style={{
          fontSize: '11px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          padding: 'var(--space-1) var(--space-2)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
          <Lightbulb size={12} />
          Serendipity
        </span>
        <button
          onClick={() => load(true)}
          disabled={refreshing}
          aria-label="Refresh connections"
          data-testid="serendipity-refresh"
          style={{
            background: 'none',
            border: 'none',
            cursor: refreshing ? 'default' : 'pointer',
            color: 'var(--color-text-muted)',
            display: 'inline-flex',
            alignItems: 'center',
            padding: '2px',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          <RefreshCw
            size={11}
            style={{ animation: refreshing ? 'spin 1s linear infinite' : 'none' }}
          />
        </button>
      </div>

      {/* Error */}
      {error && (
        <div
          style={{
            padding: 'var(--space-2)',
            fontSize: '11px',
            color: 'var(--color-danger, #c0392b)',
          }}
        >
          {error}
        </div>
      )}

      {/* No data */}
      {!error && (!data || data.pairs.length === 0) && (
        <div
          style={{
            padding: 'var(--space-2)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
          }}
        >
          No unexpected connections found
        </div>
      )}

      {/* Connection cards */}
      {data &&
        data.pairs.map((pair, idx) => (
          <div
            key={idx}
            data-testid="serendipity-card"
            style={{
              padding: 'var(--space-2)',
              borderBottom:
                idx < data.pairs.length - 1 ? '1px solid var(--color-border)' : 'none',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 'var(--space-2)',
              }}
            >
              <Link2
                size={12}
                style={{
                  color: 'var(--color-accent)',
                  marginTop: '2px',
                  flexShrink: 0,
                }}
              />
              <div style={{ flex: 1, minWidth: 0 }}>
                {/* Score badge */}
                <div
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '4px',
                    fontSize: '10px',
                    fontWeight: 600,
                    color:
                      pair.score >= 0.7
                        ? 'var(--color-success, #10b981)'
                        : pair.score >= 0.4
                          ? 'var(--color-warning, #f59e0b)'
                          : 'var(--color-text-muted)',
                    marginBottom: '2px',
                  }}
                >
                  {(pair.score * 100).toFixed(0)}% match
                </div>

                {/* Reason */}
                <div
                  style={{
                    fontSize: '11px',
                    color: 'var(--color-text-secondary)',
                    lineHeight: 1.4,
                  }}
                >
                  {pair.reason}
                </div>

                {/* Block IDs */}
                <div
                  style={{
                    marginTop: '4px',
                    fontSize: '10px',
                    color: 'var(--color-text-muted)',
                    fontFamily: 'monospace',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {pair.block_a.slice(0, 8)}… ↔ {pair.block_b.slice(0, 8)}…
                </div>
              </div>
            </div>
          </div>
        ))}

      {/* Show count if truncated */}
      {data && data.pairs.length === limit && (
        <div
          style={{
            padding: 'var(--space-2)',
            fontSize: '10px',
            color: 'var(--color-text-muted)',
            fontStyle: 'italic',
            textAlign: 'center',
          }}
        >
          Showing {limit} of {data.pairs.length}+ connections
        </div>
      )}
    </div>
  )
}
