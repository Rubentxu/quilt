// ─── SemanticInsight — cognitive panel (semantic, read-only) ─────
//
// Per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`:
//   - "Significado de conexiones conceptuales. Lo provee el agente
//      externo — Quilt solo muestra."
//
// This panel is a PASSIVE reader. It filters the current page's
// blocks for the convention `type:: insight` and renders them in
// chronological order (most recent first). Quilt does not write
// insight blocks; agents do (per ADR-0001, no internal AI).
//
// When the `created_by` property is present and starts with
// `agent::`, we surface the author so the user can audit which
// agent produced the insight.

import { useCallback, useEffect, useState } from 'react'
import { Sparkles, RefreshCw, Loader2 } from 'lucide-react'
import { api } from '@core/api-client'
import type { Block } from '@shared/types/api'

interface SemanticInsightProps {
  pageName: string | null
  isOpen: boolean
}

interface InsightItem {
  block: Block
  author: string | null
}

const INSIGHT_TYPE = 'insight'

function pickAuthor(block: Block): string | null {
  const props = block.properties ?? []
  const createdBy = props.find((p) => p.key === 'created_by')?.value
  if (createdBy == null) return null
  const s = String(createdBy)
  return s.length > 0 ? s : null
}

function isInsightBlock(block: Block): boolean {
  const props = block.properties ?? []
  const typeProp = props.find((p) => p.key === 'type')?.value
  return typeProp != null && String(typeProp) === INSIGHT_TYPE
}

export function SemanticInsight({ pageName, isOpen }: SemanticInsightProps) {
  const [items, setItems] = useState<InsightItem[]>([])
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
        const blocks = await api.getPageBlocks(pageName)
        // Filter for `type:: insight`; sort by `updatedAt` desc.
        const filtered = blocks
          .filter(isInsightBlock)
          .sort(
            (a, b) =>
              new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime(),
          )
          .map((b) => ({ block: b, author: pickAuthor(b) }))
        setItems(filtered)
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

  if (!isOpen || !pageName) return null

  return (
    <div data-testid="semantic-insight" style={{ padding: 'var(--space-2)' }}>
      <div
        data-testid="semantic-insight-header"
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
          <Sparkles size={12} /> Semantic Insight
        </span>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh semantic insights"
          data-testid="semantic-insight-refresh"
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
          data-testid="semantic-insight-loading"
        >
          <Loader2
            size={12}
            style={{ verticalAlign: 'middle', marginRight: '4px' }}
          />
          Loading insights…
        </div>
      )}

      {error && (
        <div
          data-testid="semantic-insight-error"
          style={{
            padding: 'var(--space-2)',
            fontSize: '11px',
            color: 'var(--color-danger, #c0392b)',
          }}
        >
          {error}
        </div>
      )}

      {!loading && !error && items.length === 0 && (
        <div
          data-testid="semantic-insight-empty"
          style={{
            padding: 'var(--space-2)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
          }}
        >
          <Sparkles
            size={12}
            style={{ verticalAlign: 'middle', marginRight: '4px' }}
          />
          No insights on this page. Agents write insight blocks with{' '}
          <code>type:: insight</code>.
        </div>
      )}

      {!loading &&
        !error &&
        items.map(({ block, author }) => {
          const preview =
            (block.content ?? '').length > 160
              ? (block.content ?? '').slice(0, 160) + '…'
              : block.content ?? '(empty)'
          return (
            <div
              key={block.id}
              data-testid={`semantic-insight-item-${block.id}`}
              style={{
                padding: 'var(--space-2)',
                borderBottom: '1px solid var(--color-border)',
                fontSize: '12px',
              }}
            >
              {author && (
                <div
                  style={{
                    color: 'var(--color-accent)',
                    fontWeight: 500,
                    fontSize: '11px',
                    marginBottom: '2px',
                  }}
                >
                  {author}
                </div>
              )}
              <div
                style={{
                  color: 'var(--color-text-secondary)',
                  lineHeight: 1.4,
                }}
              >
                {preview}
              </div>
              <div
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '10px',
                  marginTop: '2px',
                }}
              >
                {new Date(block.updatedAt).toLocaleString()}
              </div>
            </div>
          )
        })}
    </div>
  )
}
