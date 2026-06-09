// ─── StructuralGraph — cognitive panel (structural) ──────────────
//
// Per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`:
//   - "Topología del grafo calculada por Quilt: conectividad, decay,
//      orphans, similitud estructural."
//   - Boundary: Quilt (Rust) computes topology; Quilt (UI) shows it.
//
// In this MVP, the Rust `quilt-analysis` engine is NOT yet exposed
// over HTTP (the `quilt-cognitive` route is a follow-up — see
// `apply-progress` of `quilt-architecture-review-c3-serialize`).
// So the panel derives a *minimal* structural view from data that
// IS available over HTTP:
//
//   - `api.getPageBlocks(name)`  → block count, property distribution
//   - `api.getPageBacklinks(name)` → incoming reference count
//
// The stats are intentionally simple: block count, property count,
// reference count, most-used property keys, and orphan detection
// (blocks with neither outgoing `[[wikilinks]]` nor incoming
// backlinks). When the structural-mirror endpoint is mounted, this
// component can be extended to call it without changing the public
// surface (`pageName`, `isOpen`).
//
// The orphan detector looks for the wikilink syntax `[[Page Name]]`
// in block content. This is a *string-level* check; it does not run
// the real link parser. False positives are acceptable for a
// preview; the structural-mirror endpoint will replace this later.

import { useCallback, useEffect, useMemo, useState } from 'react'
import { Network, RefreshCw, Loader2 } from 'lucide-react'
import { api } from '@core/api-client'
import type { Block, Backlink } from '@shared/types/api'

interface StructuralGraphProps {
  pageName: string | null
  isOpen: boolean
}

interface PageStats {
  blockCount: number
  propertyCount: number
  referenceCount: number
  /** Property key → number of blocks that carry it. */
  propertyHistogram: Array<{ key: string; count: number }>
  /** IDs of orphan blocks (no incoming backlinks AND no outgoing [[wikilinks]]). */
  orphanIds: string[]
}

// Note: no `g` flag — `RegExp.prototype.test` with `g` is stateful
// (advances `lastIndex` across calls) and would give wrong results
// when called repeatedly on different strings. The check is a simple
// substring search; we don't need the match positions.
const WIKILINK_RE = /\[\[[^\]]+\]\]/

function detectWikilink(content: string): boolean {
  return WIKILINK_RE.test(content)
}

function computeStats(blocks: Block[], backlinks: Backlink[]): PageStats {
  const propertyHistogram = new Map<string, number>()
  let propertyCount = 0

  for (const block of blocks) {
    const props = block.properties ?? []
    for (const p of props) {
      propertyCount += 1
      propertyHistogram.set(p.key, (propertyHistogram.get(p.key) ?? 0) + 1)
    }
  }

  // The `Backlink` DTO only gives page-level data — it does NOT
  // tell us which block on the *current* page each link points to.
  // So we use a page-level proxy: if the page has zero incoming
  // backlinks AND a block has no outgoing `[[wikilinks]]`, the
  // block is a candidate orphan. When the structural-mirror
  // endpoint is mounted (see the `quilt-cognitive` follow-up),
  // this can be replaced with a per-block incoming-edge set
  // without changing the panel's public surface.
  const pageHasIncomingRefs = backlinks.length > 0

  const orphanIds: string[] = []
  for (const block of blocks) {
    const hasOutgoingLink = detectWikilink(block.content ?? '')
    if (!hasOutgoingLink && !pageHasIncomingRefs) {
      orphanIds.push(block.id)
    }
  }

  return {
    blockCount: blocks.length,
    propertyCount,
    referenceCount: backlinks.length,
    propertyHistogram: Array.from(propertyHistogram.entries())
      .map(([key, count]) => ({ key, count }))
      .sort((a, b) => b.count - a.count || a.key.localeCompare(b.key)),
    orphanIds,
  }
}

export function StructuralGraph({ pageName, isOpen }: StructuralGraphProps) {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [backlinks, setBacklinks] = useState<Backlink[]>([])
  const [loading, setLoading] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(
    async (isRefresh: boolean) => {
      if (!pageName) return
      if (isRefresh) setRefreshing(true)
      else setLoading(true)
      setError(null)
      try {
        const [b, bl] = await Promise.all([
          api.getPageBlocks(pageName),
          api.getPageBacklinks(pageName),
        ])
        setBlocks(b)
        setBacklinks(bl)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load')
      } finally {
        setLoading(false)
        setRefreshing(false)
      }
    },
    [pageName],
  )

  useEffect(() => {
    if (!isOpen || !pageName) return
    void load(false)
  }, [isOpen, pageName, load])

  const stats = useMemo(() => computeStats(blocks, backlinks), [blocks, backlinks])

  const orphanBlocksById = useMemo(() => {
    const map = new Map<string, Block>()
    for (const id of stats.orphanIds) {
      const block = blocks.find((b) => b.id === id)
      if (block) map.set(id, block)
    }
    return map
  }, [stats.orphanIds, blocks])

  if (!isOpen || !pageName) return null

  return (
    <div data-testid="structural-graph" style={{ padding: 'var(--space-2)' }}>
      <div
        data-testid="structural-graph-header"
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
        <span
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
          }}
        >
          <Network size={12} /> Structural Graph
        </span>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh structural graph"
          data-testid="structural-graph-refresh"
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
            style={{
              animation: refreshing ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {loading && (
        <div
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
          }}
          data-testid="structural-graph-loading"
        >
          <Loader2
            size={12}
            style={{ verticalAlign: 'middle', marginRight: '4px' }}
          />
          Loading structural graph…
        </div>
      )}

      {error && (
        <div
          data-testid="structural-graph-error"
          style={{
            padding: 'var(--space-2)',
            fontSize: '11px',
            color: 'var(--color-danger, #c0392b)',
          }}
        >
          {error}
        </div>
      )}

      {!loading && !error && (
        <>
          {/* Stat tiles — three counts side by side */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              gap: 'var(--space-2)',
              padding: 'var(--space-2)',
            }}
          >
            <StatTile
              testId="structural-graph-block-count"
              label="Blocks"
              value={stats.blockCount}
            />
            <StatTile
              testId="structural-graph-property-count"
              label="Properties"
              value={stats.propertyCount}
            />
            <StatTile
              testId="structural-graph-reference-count"
              label="References"
              value={stats.referenceCount}
            />
          </div>

          {/* Most-used properties */}
          <div
            style={{
              padding: 'var(--space-1) var(--space-2) var(--space-2)',
              borderTop: '1px solid var(--color-border)',
            }}
          >
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                color: 'var(--color-text-muted)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                padding: 'var(--space-2) 0',
              }}
            >
              Most used properties
            </div>
            {stats.propertyHistogram.length === 0 ? (
              <div
                data-testid="structural-graph-property-empty"
                style={{
                  fontSize: '12px',
                  color: 'var(--color-text-muted)',
                  fontStyle: 'italic',
                }}
              >
                No properties on this page.
              </div>
            ) : (
              <ul
                data-testid="structural-graph-property-list"
                style={{
                  listStyle: 'none',
                  padding: 0,
                  margin: 0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '2px',
                }}
              >
                {stats.propertyHistogram.map(({ key, count }) => (
                  <li
                    key={key}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      fontSize: '12px',
                      color: 'var(--color-text-secondary)',
                      padding: 'var(--space-1) 0',
                    }}
                  >
                    <span
                      style={{
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {key}
                    </span>
                    <span
                      style={{
                        fontSize: '10px',
                        color: 'var(--color-text-muted)',
                        background: 'var(--color-surface-subtle)',
                        borderRadius: 'var(--radius-pill)',
                        padding: '0 6px',
                        lineHeight: '16px',
                        marginLeft: 'var(--space-2)',
                        flexShrink: 0,
                      }}
                    >
                      {count}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Orphan detection */}
          <div
            style={{
              padding: 'var(--space-1) var(--space-2) var(--space-2)',
              borderTop: '1px solid var(--color-border)',
            }}
          >
            <div
              style={{
                fontSize: '10px',
                fontWeight: 600,
                color: 'var(--color-text-muted)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                padding: 'var(--space-2) 0',
              }}
            >
              Orphans ({orphanBlocksById.size})
            </div>
            {orphanBlocksById.size === 0 ? (
              <div
                data-testid="structural-graph-orphans"
                style={{
                  fontSize: '12px',
                  color: 'var(--color-text-muted)',
                  fontStyle: 'italic',
                }}
              >
                No orphan blocks.
              </div>
            ) : (
              <ul
                data-testid="structural-graph-orphans"
                style={{
                  listStyle: 'none',
                  padding: 0,
                  margin: 0,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '2px',
                }}
              >
                {Array.from(orphanBlocksById.values()).map((block) => {
                  const preview =
                    (block.content ?? '').length > 80
                      ? (block.content ?? '').slice(0, 80) + '…'
                      : block.content ?? '(empty)'
                  return (
                    <li
                      key={block.id}
                      data-testid={`structural-graph-orphan-${block.id}`}
                      style={{
                        fontSize: '12px',
                        color: 'var(--color-text-secondary)',
                        padding: 'var(--space-1) 0',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {preview}
                    </li>
                  )
                })}
              </ul>
            )}
          </div>
        </>
      )}
    </div>
  )
}

interface StatTileProps {
  label: string
  value: number
  testId: string
}

function StatTile({ label, value, testId }: StatTileProps) {
  return (
    <div
      data-testid={testId}
      style={{
        background: 'var(--color-surface-subtle)',
        borderRadius: 'var(--radius-md)',
        padding: 'var(--space-2)',
        textAlign: 'center',
      }}
    >
      <div
        style={{
          fontSize: '16px',
          fontWeight: 700,
          color: 'var(--color-text-primary)',
        }}
      >
        {value}
      </div>
      <div
        style={{
          fontSize: '10px',
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          marginTop: '2px',
        }}
      >
        {label}
      </div>
    </div>
  )
}
