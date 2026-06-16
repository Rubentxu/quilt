// ─── CognitiveGraph — cognitive panel (graph view) ─────────────────────────
//
// CG-2: Cognitive Dashboard / Graph View.
// Displays the global knowledge graph with clusters, frontier nodes
// (highly connected hubs), and gap nodes (isolated orphans).
//
// Per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`:
//   - "Topología del grafo calculada por Quilt: conectividad, decay,
//      orphans, similitud estructural."
//   - Boundary: Quilt (Rust) computes topology; Quilt (UI) shows it.
//
// The Rust `CognitiveDashboardService` in `quilt-analysis` produces the
// graph data (clusters, frontier nodes, gap nodes, edges) and this
// component renders it as an interactive panel.

import { useCallback, useEffect, useMemo, useState } from 'react'
import { Network, RefreshCw, Loader2, AlertCircle, GitBranch, Zap, AlertTriangle } from 'lucide-react'
import { api } from '@core/api-client'
import type { CognitiveGraphDto, CognitiveGraphNode, CognitiveGraphCluster } from '@shared/types/api'

interface CognitiveGraphProps {
  /** Called when the user wants to open a block. */
  onNavigate?: (blockId: string, pageName: string | null) => void
}

// ─── Cluster color palette ─────────────────────────────────────────────────

const CLUSTER_COLORS = [
  'var(--color-accent, #6366f1)',
  'var(--color-success, #22c55e)',
  'var(--color-warning, #e67e22)',
  '#8b5cf6', // purple
  '#06b6d4', // cyan
  '#ec4899', // pink
  '#f59e0b', // amber
]

function clusterColor(clusterId: string | null, fallbackIndex: number): string {
  if (!clusterId) return 'var(--color-text-muted)'
  // Derive a stable color from the cluster id string
  const hash = clusterId.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0)
  return CLUSTER_COLORS[hash % CLUSTER_COLORS.length]
}

// ─── Node badge helpers ───────────────────────────────────────────────────

function FrontierBadge() {
  return (
    <span
      data-testid="badge-frontier"
      title="Highly connected hub"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '2px',
        fontSize: '9px',
        fontWeight: 700,
        color: 'var(--color-accent)',
        background: 'color-mix(in srgb, var(--color-accent) 15%, transparent)',
        borderRadius: 'var(--radius-pill)',
        padding: '1px 5px',
        letterSpacing: '0.04em',
        textTransform: 'uppercase',
      }}
    >
      <Zap size={8} /> frontier
    </span>
  )
}

function GapBadge() {
  return (
    <span
      data-testid="badge-gap"
      title="Isolated — no connections"
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '2px',
        fontSize: '9px',
        fontWeight: 700,
        color: 'var(--color-warning, #e67e22)',
        background: 'color-mix(in srgb, var(--color-warning, #e67e22) 15%, transparent)',
        borderRadius: 'var(--radius-pill)',
        padding: '1px 5px',
        letterSpacing: '0.04em',
        textTransform: 'uppercase',
      }}
    >
      <AlertTriangle size={8} /> gap
    </span>
  )
}

// ─── Empty state ─────────────────────────────────────────────────────────

function CognitiveGraphEmpty() {
  return (
    <div
      data-testid="cognitive-graph-empty"
      style={{
        padding: 'var(--space-3)',
        color: 'var(--color-text-muted)',
        fontSize: '12px',
        fontStyle: 'italic',
        textAlign: 'center',
      }}
    >
      No graph data yet — create some pages and link blocks to see the graph.
    </div>
  )
}

// ─── Error state ─────────────────────────────────────────────────────────

interface ErrorStateProps {
  message: string
  onRetry: () => void
}

function CognitiveGraphError({ message, onRetry }: ErrorStateProps) {
  return (
    <div
      data-testid="cognitive-graph-error"
      style={{
        padding: 'var(--space-3)',
        color: 'var(--color-danger, #c0392b)',
        fontSize: '12px',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 'var(--space-2)',
      }}
    >
      <AlertCircle size={16} />
      <span>{message}</span>
      <button
        onClick={onRetry}
        data-testid="cognitive-graph-retry"
        style={{
          background: 'none',
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-sm)',
          padding: '4px 12px',
          fontSize: '11px',
          cursor: 'pointer',
          color: 'var(--color-text-secondary)',
        }}
      >
        Retry
      </button>
    </div>
  )
}

// ─── Stats bar ──────────────────────────────────────────────────────────

interface StatsBarProps {
  nodeCount: number
  edgeCount: number
  clusterCount: number
  frontierCount: number
  gapCount: number
}

function StatsBar({ nodeCount, edgeCount, clusterCount, frontierCount, gapCount }: StatsBarProps) {
  return (
    <div
      data-testid="cognitive-graph-stats"
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(5, 1fr)',
        gap: 'var(--space-1)',
        padding: 'var(--space-2)',
        borderBottom: '1px solid var(--color-border)',
      }}
    >
      <Stat label="Nodes" value={nodeCount} testId="stat-nodes" />
      <Stat label="Edges" value={edgeCount} testId="stat-edges" />
      <Stat label="Clusters" value={clusterCount} testId="stat-clusters" />
      <Stat label="Frontier" value={frontierCount} testId="stat-frontier" accent />
      <Stat label="Gaps" value={gapCount} testId="stat-gaps" accent />
    </div>
  )
}

interface StatProps {
  label: string
  value: number
  testId: string
  accent?: boolean
}

function Stat({ label, value, testId, accent }: StatProps) {
  return (
    <div
      data-testid={testId}
      style={{
        textAlign: 'center',
        background: accent ? 'color-mix(in srgb, var(--color-accent) 8%, transparent)' : 'var(--color-surface-subtle)',
        borderRadius: 'var(--radius-md)',
        padding: 'var(--space-1)',
      }}
    >
      <div
        style={{
          fontSize: '14px',
          fontWeight: 700,
          color: accent ? 'var(--color-accent)' : 'var(--color-text-primary)',
        }}
      >
        {value}
      </div>
      <div
        style={{
          fontSize: '9px',
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.04em',
        }}
      >
        {label}
      </div>
    </div>
  )
}

// ─── Node card ──────────────────────────────────────────────────────────

interface NodeCardProps {
  node: CognitiveGraphNode
  color: string
  onNavigate?: (blockId: string, pageName: string | null) => void
}

function NodeCard({ node, color, onNavigate }: NodeCardProps) {
  const handleClick = useCallback(() => {
    onNavigate?.(node.blockId, node.pageName)
  }, [onNavigate, node.blockId, node.pageName])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if ((e.key === 'Enter' || e.key === ' ') && onNavigate) {
        e.preventDefault()
        onNavigate(node.blockId, node.pageName)
      }
    },
    [onNavigate, node.blockId, node.pageName],
  )

  const preview =
    node.contentPreview.length > 80
      ? node.contentPreview.slice(0, 80) + '…'
      : node.contentPreview || '(empty)'

  return (
    <div
      data-testid={`node-${node.id}`}
      role={onNavigate ? 'button' : undefined}
      tabIndex={onNavigate ? 0 : undefined}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      aria-label={
        onNavigate
          ? `Open block: ${preview} on page ${node.pageName}${node.isFrontier ? ', frontier node' : ''}${node.isGap ? ', gap node' : ''}`
          : undefined
      }
      style={{
        padding: 'var(--space-2)',
        borderBottom: '1px solid var(--color-border)',
        cursor: onNavigate ? 'pointer' : 'default',
        borderLeft: `3px solid ${color}`,
        background: node.isFrontier
          ? 'color-mix(in srgb, var(--color-accent) 5%, transparent)'
          : node.isGap
            ? 'color-mix(in srgb, var(--color-warning, #e67e22) 5%, transparent)'
            : 'transparent',
      }}
    >
      {/* Header row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 'var(--space-1)',
          marginBottom: '4px',
        }}
      >
        <span
          style={{
            fontSize: '10px',
            color: 'var(--color-text-muted)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            flex: 1,
          }}
          title={node.pageName}
        >
          {node.pageName}
        </span>
        <div style={{ display: 'flex', gap: '3px', flexShrink: 0 }}>
          {node.isFrontier && <FrontierBadge />}
          {node.isGap && <GapBadge />}
        </div>
      </div>

      {/* Content preview */}
      <div
        style={{
          fontSize: '12px',
          color: 'var(--color-text-secondary)',
          lineHeight: 1.4,
          marginBottom: '4px',
        }}
      >
        {preview}
      </div>

      {/* Footer: cluster + influence */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 'var(--space-1)',
        }}
      >
        {node.clusterId && (
          <span
            style={{
              fontSize: '9px',
              color: color,
              fontWeight: 600,
              display: 'flex',
              alignItems: 'center',
              gap: '2px',
            }}
          >
            <GitBranch size={8} />
            {node.clusterId}
          </span>
        )}
        <span
          style={{
            fontSize: '9px',
            color: 'var(--color-text-muted)',
            marginLeft: 'auto',
          }}
        >
          influence {(node.influenceScore * 100).toFixed(0)}%
        </span>
      </div>
    </div>
  )
}

// ─── Cluster section ─────────────────────────────────────────────────────

interface ClusterSectionProps {
  clusters: CognitiveGraphCluster[]
  nodeMap: Map<string, CognitiveGraphNode>
  onNavigate?: (blockId: string, pageName: string | null) => void
}

function ClusterSection({ clusters, nodeMap, onNavigate }: ClusterSectionProps) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div style={{ borderBottom: '1px solid var(--color-border)' }}>
      {/* Header */}
      <button
        data-testid="cluster-toggle"
        onClick={() => setExpanded((e) => !e)}
        aria-expanded={expanded}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: 'var(--space-2)',
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          fontSize: '11px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          <GitBranch size={11} />
          Clusters ({clusters.length})
        </span>
        <span style={{ fontSize: '10px' }}>{expanded ? '▲' : '▼'}</span>
      </button>

      {/* Cluster list */}
      {expanded && (
        <div style={{ padding: '0 var(--space-2) var(--space-2)' }}>
          {clusters.length === 0 ? (
            <div
              data-testid="clusters-empty"
              style={{ fontSize: '11px', color: 'var(--color-text-muted)', fontStyle: 'italic' }}
            >
              No clusters detected.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
              {clusters.map((cluster, i) => {
                const color = clusterColor(cluster.id, i)
                const clusterNodes = cluster.blockIds
                  .map((id) => nodeMap.get(id))
                  .filter(Boolean) as CognitiveGraphNode[]

                return (
                  <div
                    key={cluster.id}
                    data-testid={`cluster-${cluster.id}`}
                    style={{
                      borderLeft: `3px solid ${color}`,
                      paddingLeft: 'var(--space-2)',
                    }}
                  >
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        marginBottom: 'var(--space-1)',
                      }}
                    >
                      <span
                        style={{ fontSize: '10px', fontWeight: 600, color: color }}
                      >
                        {cluster.id}
                      </span>
                      <span style={{ fontSize: '9px', color: 'var(--color-text-muted)' }}>
                        {cluster.coherenceScore.toFixed(2)} coherence · {cluster.blockIds.length} nodes
                      </span>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1px' }}>
                      {clusterNodes.slice(0, 3).map((node) => (
                        <div
                          key={node.id}
                          data-testid={`cluster-node-${node.id}`}
                          role={onNavigate ? 'button' : undefined}
                          tabIndex={onNavigate ? 0 : undefined}
                          onClick={() => onNavigate?.(node.blockId, node.pageName)}
                          onKeyDown={(e) => {
                            if ((e.key === 'Enter' || e.key === ' ') && onNavigate) {
                              e.preventDefault()
                              onNavigate(node.blockId, node.pageName)
                            }
                          }}
                          style={{
                            fontSize: '11px',
                            color: 'var(--color-text-secondary)',
                            cursor: onNavigate ? 'pointer' : 'default',
                            padding: '2px 4px',
                            borderRadius: 'var(--radius-sm)',
                            background: 'var(--color-surface-subtle)',
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                          title={node.contentPreview}
                        >
                          {node.contentPreview || '(empty)'}
                        </div>
                      ))}
                      {clusterNodes.length > 3 && (
                        <div
                          style={{
                            fontSize: '10px',
                            color: 'var(--color-text-muted)',
                            fontStyle: 'italic',
                          }}
                        >
                          +{clusterNodes.length - 3} more
                        </div>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

// ─── Main component ─────────────────────────────────────────────────────

export function CognitiveGraph({ onNavigate }: CognitiveGraphProps) {
  const [data, setData] = useState<CognitiveGraphDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      const result = await api.getCognitiveGraph()
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load cognitive graph')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    void load(false)
  }, [load])

  // Build a node lookup map
  const nodeMap = useMemo(() => {
    const map = new Map<string, CognitiveGraphNode>()
    if (!data) return map
    for (const node of data.nodes) {
      map.set(node.id, node)
    }
    return map
  }, [data])

  return (
    <div
      data-testid="cognitive-graph"
      style={{
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        maxHeight: '480px',
      }}
      role="region"
      aria-label="Cognitive Graph"
    >
      {/* Header */}
      <div
        style={{
          padding: 'var(--space-2) var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Network size={14} color="var(--color-accent)" />
          <span style={{ fontWeight: 600, fontSize: '13px' }}>Cognitive Graph</span>
        </div>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh cognitive graph"
          data-testid="cognitive-graph-refresh"
          style={{
            background: 'none',
            border: 'none',
            cursor: refreshing ? 'default' : 'pointer',
            color: 'var(--color-text-muted)',
            display: 'inline-flex',
            alignItems: 'center',
            padding: '4px',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          <RefreshCw
            size={13}
            style={{
              animation: refreshing ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {/* Loading state */}
      {loading && (
        <div
          data-testid="cognitive-graph-loading"
          style={{
            padding: 'var(--space-4)',
            textAlign: 'center',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 'var(--space-2)',
          }}
        >
          <Loader2
            size={16}
            style={{ animation: 'spin 1s linear infinite' }}
          />
          Loading cognitive graph…
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <CognitiveGraphError message={error} onRetry={() => void load(false)} />
      )}

      {/* Content */}
      {!loading && !error && data && (
        <div style={{ flex: 1, overflow: 'auto' }}>
          {data.nodes.length === 0 ? (
            <CognitiveGraphEmpty />
          ) : (
            <>
              {/* Stats bar */}
              <StatsBar
                nodeCount={data.nodes.length}
                edgeCount={data.edges.length}
                clusterCount={data.clusters.length}
                frontierCount={data.frontierNodes.length}
                gapCount={data.gapNodes.length}
              />

              {/* Cluster section */}
              {data.clusters.length > 0 && (
                <ClusterSection
                  clusters={data.clusters}
                  nodeMap={nodeMap}
                  onNavigate={onNavigate}
                />
              )}

              {/* Frontier nodes section */}
              {data.frontierNodes.length > 0 && (
                <div style={{ borderBottom: '1px solid var(--color-border)' }}>
                  <div
                    style={{
                      padding: 'var(--space-2) var(--space-3)',
                      fontSize: '10px',
                      fontWeight: 600,
                      color: 'var(--color-accent)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.05em',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                    }}
                  >
                    <Zap size={10} /> Frontier hubs ({data.frontierNodes.length})
                  </div>
                  {data.frontierNodes.map((id) => {
                    const node = nodeMap.get(id)
                    if (!node) return null
                    return (
                      <NodeCard
                        key={id}
                        node={node}
                        color={clusterColor(node.clusterId, 0)}
                        onNavigate={onNavigate}
                      />
                    )
                  })}
                </div>
              )}

              {/* Gap nodes section */}
              {data.gapNodes.length > 0 && (
                <div style={{ borderBottom: '1px solid var(--color-border)' }}>
                  <div
                    style={{
                      padding: 'var(--space-2) var(--space-3)',
                      fontSize: '10px',
                      fontWeight: 600,
                      color: 'var(--color-warning, #e67e22)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.05em',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                    }}
                  >
                    <AlertTriangle size={10} /> Gap / isolated ({data.gapNodes.length})
                  </div>
                  {Array.from(new Set(data.gapNodes)).map((id) => {
                    const node = nodeMap.get(id)
                    if (!node) return null
                    return (
                      <NodeCard
                        key={id}
                        node={node}
                        color={clusterColor(node.clusterId, 0)}
                        onNavigate={onNavigate}
                      />
                    )
                  })}
                </div>
              )}

              {/* All nodes section (paginated) — excludes nodes already shown in frontier/gap sections */}
              <div>
                <div
                  style={{
                    padding: 'var(--space-2) var(--space-3)',
                    fontSize: '10px',
                    fontWeight: 600,
                    color: 'var(--color-text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                  }}
                >
                  All nodes ({data.nodes.length})
                </div>
                {data.nodes
                  .filter((n) => !n.isFrontier && !n.isGap)
                  .map((node) => (
                    <NodeCard
                      key={node.id}
                      node={node}
                      color={clusterColor(node.clusterId, 0)}
                      onNavigate={onNavigate}
                    />
                  ))}
              </div>

              {/* Generated at footer */}
              <div
                data-testid="cognitive-graph-footer"
                style={{
                  padding: 'var(--space-2)',
                  borderTop: '1px solid var(--color-border)',
                  color: 'var(--color-text-muted)',
                  fontSize: '10px',
                  textAlign: 'center',
                }}
              >
                Graph generated {new Date(data.generatedAt).toLocaleString()}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}
