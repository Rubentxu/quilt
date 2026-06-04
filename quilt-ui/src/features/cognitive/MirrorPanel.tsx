/**
 * MirrorPanel — G7 Dream Cycle Display
 *
 * Displays structural mirror analysis: clusters, gaps, frontiers, and density.
 * Auto-refreshes every 5 minutes.
 */

import { useState, useEffect, useCallback } from 'react'
import { RefreshCw, Loader2, Network, AlertTriangle, Target } from 'lucide-react'
import { api, type MirrorAnalysisDto } from '@core/api-client'
import { useRefreshInterval } from './useRefreshInterval'

interface MirrorPanelProps {
  /** Auto-refresh interval in ms. Default 5 minutes (300000). 0 to disable. */
  refreshInterval?: number
}

export function MirrorPanel({ refreshInterval = 300000 }: MirrorPanelProps) {
  const [data, setData] = useState<MirrorAnalysisDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)

    try {
      const result = await api.getAnalysisMirror()
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  // Initial load
  useEffect(() => {
    void load(false)
  }, [load])

  // Auto-refresh
  useRefreshInterval(() => load(true), refreshInterval)

  if (loading) {
    return (
      <div
        data-testid="mirror-panel"
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
        Loading mirror analysis…
      </div>
    )
  }

  return (
    <div data-testid="mirror-panel" style={{ padding: 'var(--space-2)' }}>
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
          <Network size={12} />
          Mirror Analysis
        </span>
        <button
          onClick={() => load(true)}
          disabled={refreshing}
          aria-label="Refresh mirror analysis"
          data-testid="mirror-refresh"
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
      {!error && (!data || (data.clusters.length === 0 && data.gaps.length === 0)) && (
        <div
          style={{
            padding: 'var(--space-2)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
          }}
        >
          No structural analysis available yet
        </div>
      )}

      {/* Density */}
      {data && data.density > 0 && (
        <div
          style={{
            padding: 'var(--space-2)',
            borderBottom: '1px solid var(--color-border)',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-2)',
              fontSize: '11px',
              color: 'var(--color-text-secondary)',
            }}
          >
            <Target size={12} style={{ color: 'var(--color-primary)' }} />
            <span>Density</span>
            <span
              style={{
                marginLeft: 'auto',
                fontWeight: 600,
                color: 'var(--color-text-primary)',
              }}
            >
              {(data.density * 100).toFixed(1)}%
            </span>
          </div>
          <div
            style={{
              marginTop: '4px',
              height: '4px',
              background: 'var(--color-surface-subtle)',
              borderRadius: '2px',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                height: '100%',
                width: `${data.density * 100}%`,
                background: 'var(--color-primary)',
                transition: 'width 0.3s ease',
              }}
            />
          </div>
        </div>
      )}

      {/* Clusters */}
      {data && data.clusters.length > 0 && (
        <div style={{ padding: 'var(--space-2)', borderBottom: '1px solid var(--color-border)' }}>
          <div
            style={{
              fontSize: '10px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
              marginBottom: 'var(--space-2)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
            }}
          >
            Clusters ({data.clusters.length})
          </div>
          {data.clusters.slice(0, 3).map((cluster, idx) => (
            <div
              key={idx}
              data-testid="mirror-cluster"
              style={{
                fontSize: '11px',
                padding: 'var(--space-1) 0',
                color: 'var(--color-text-secondary)',
              }}
            >
              {cluster.theme ? (
                <span style={{ fontWeight: 500 }}>{cluster.theme}</span>
              ) : (
                <span style={{ color: 'var(--color-text-muted)', fontStyle: 'italic' }}>
                  Unnamed cluster
                </span>
              )}
              <span style={{ marginLeft: 'var(--space-2)', color: 'var(--color-text-muted)' }}>
                {cluster.block_ids.length} blocks · {cluster.coherence_score.toFixed(2)} coherence
              </span>
            </div>
          ))}
          {data.clusters.length > 3 && (
            <div
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
                fontStyle: 'italic',
              }}
            >
              +{data.clusters.length - 3} more clusters
            </div>
          )}
        </div>
      )}

      {/* Gaps */}
      {data && data.gaps.length > 0 && (
        <div style={{ padding: 'var(--space-2)' }}>
          <div
            style={{
              fontSize: '10px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
              marginBottom: 'var(--space-2)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
          >
            <AlertTriangle size={10} style={{ color: 'var(--color-warning, #f59e0b)' }} />
            Gaps ({data.gaps.length})
          </div>
          {data.gaps.slice(0, 3).map((gap, idx) => (
            <div
              key={idx}
              data-testid="mirror-gap"
              style={{
                fontSize: '11px',
                padding: 'var(--space-1) 0',
                color: 'var(--color-text-secondary)',
              }}
            >
              <span style={{ color: 'var(--color-text-muted)' }}>Missing connection</span>
              <span style={{ marginLeft: 'var(--space-1)' }}>
                {gap.shared_refs.length} shared refs
              </span>
            </div>
          ))}
          {data.gaps.length > 3 && (
            <div
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
                fontStyle: 'italic',
              }}
            >
              +{data.gaps.length - 3} more gaps
            </div>
          )}
        </div>
      )}

      {/* Frontiers */}
      {data && data.frontiers.length > 0 && (
        <div
          style={{
            padding: 'var(--space-2)',
            borderTop: '1px solid var(--color-border)',
          }}
        >
          <div
            style={{
              fontSize: '10px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
              marginBottom: 'var(--space-1)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
            }}
          >
            Frontiers ({data.frontiers.length})
          </div>
          <div
            style={{
              fontSize: '10px',
              color: 'var(--color-text-muted)',
            }}
          >
            {data.frontiers.slice(0, 5).join(', ')}
            {data.frontiers.length > 5 && ` +${data.frontiers.length - 5} more`}
          </div>
        </div>
      )}
    </div>
  )
}
