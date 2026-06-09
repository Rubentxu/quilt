import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Bot, RefreshCw, Loader2 } from 'lucide-react'
import { api } from '@core/api-client'
import type { Block } from '@shared/types/api'

// ─── AgentActivityFeed ──────────────────────────────────────────────
//
// ADR-0003 — passive view of recent agent-authored blocks. This is a
// "minimum viable" cognitive feature: it surfaces what AI agents have
// added to the graph recently so the user has a single place to audit
// them. It does NOT subscribe to real-time events yet — it refreshes
// on demand. A real-time stream will land later once the cognitive
// feature is enabled by default.
//
// Per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`
// (Q013-P2) this panel belongs to the `cognitivo::` family as
// `AgentActivityFeed`. The previous name (`AgentActivityPanel`) was
// ambiguous; "Feed" signals that this is a stream of activity items,
// not a control surface.
//
// Convention: blocks where `created_by` starts with `agent::` (e.g.
// `agent::claude`, `agent::gemini`). User-authored blocks are hidden
// by design — those live in regular block views.
//
// S2-02 — the set of agent identifiers is discovered dynamically via
// `GET /api/v1/blocks/authors` instead of being hardcoded here. New
// agents (e.g. `agent::deepseek`) show up automatically as soon as
// the first block authored by them is created. The hardcoded list
// used to miss them silently.

interface ActivityItem {
  block: Block
  pageName: string | null
}

interface AgentActivityFeedProps {
  /** Maximum number of items to surface. Default 20. */
  maxItems?: number
}

export function AgentActivityFeed({ maxItems = 20 }: AgentActivityFeedProps) {
  const [items, setItems] = useState<ActivityItem[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const navigate = useNavigate()

  const load = useCallback(
    async (isRefresh: boolean) => {
      if (isRefresh) setRefreshing(true)
      else setLoading(true)
      setError(null)
      try {
        // S2-02: discover the agent roster from the server instead
        // of hardcoding it. The endpoint returns the distinct set
        // of `created_by` values that start with `agent::`, sorted
        // ASC. An empty array is the legitimate cold-start state —
        // we render the "No agent activity yet" placeholder in that
        // case (see the empty-state branch below).
        const agents = await api.getDistinctAuthors()
        const collected: ActivityItem[] = []
        for (const author of agents) {
          try {
            const blocks = await api.listBlocksByAuthor(author, 50)
            for (const b of blocks) {
              collected.push({ block: b, pageName: b.pageName ?? null })
            }
          } catch {
            // Per-author failure is non-fatal — we just skip that
            // author. The roster itself already loaded successfully;
            // a transient hiccup on one author shouldn't blank the
            // whole feed.
          }
        }
        // Sort by updatedAt desc and dedupe by block id.
        const seen = new Set<string>()
        collected.sort(
          (a, b) =>
            new Date(b.block.updatedAt).getTime() -
            new Date(a.block.updatedAt).getTime(),
        )
        const deduped = collected.filter((it) => {
          if (seen.has(it.block.id)) return false
          seen.add(it.block.id)
          return true
        })
        setItems(deduped.slice(0, maxItems))
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load')
      } finally {
        setLoading(false)
        setRefreshing(false)
      }
    },
    [maxItems],
  )

  useEffect(() => {
    void load(false)
  }, [load])

  function handleClick(item: ActivityItem) {
    if (item.pageName) {
      navigate({ to: '/page/$name', params: { name: item.pageName } })
    }
  }

  if (loading) {
    return (
      <div
        data-testid="agent-activity-feed"
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
        Loading agent activity…
      </div>
    )
  }

  return (
    <div
      data-testid="agent-activity-feed"
      style={{ padding: 'var(--space-2)' }}
    >
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
        <span
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
          }}
        >
          <Bot size={12} /> Agent Activity
        </span>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh agent activity"
          data-testid="agent-activity-refresh"
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

      {!error && items.length === 0 && (
        <div
          style={{
            padding: 'var(--space-2)',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            fontStyle: 'italic',
          }}
        >
          <Bot
            size={12}
            style={{ verticalAlign: 'middle', marginRight: '4px' }}
          />
          No agent activity yet
        </div>
      )}

      {items.map((item) => {
        const createdBy = item.block.properties?.find(
          (p) => p.key === 'created_by',
        )?.value
        const author = createdBy == null ? 'agent::?' : String(createdBy)
        const preview =
          item.block.content.length > 80
            ? item.block.content.slice(0, 80) + '…'
            : item.block.content
        return (
          <div
            key={item.block.id}
            onClick={() => handleClick(item)}
            data-testid="agent-activity-item"
            style={{
              padding: 'var(--space-2)',
              borderBottom: '1px solid var(--color-border)',
              fontSize: '12px',
              cursor: item.pageName ? 'pointer' : 'default',
              transition: 'background var(--motion-fast)',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'var(--color-surface-subtle)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent'
            }}
          >
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
            <div
              style={{
                color: 'var(--color-text-secondary)',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                lineHeight: 1.4,
              }}
            >
              {preview || '(empty block)'}
            </div>
            <div
              style={{
                color: 'var(--color-text-muted)',
                fontSize: '10px',
                marginTop: '2px',
              }}
            >
              {item.pageName ? (
                <span>
                  in <strong>{item.pageName}</strong>
                </span>
              ) : (
                <span>in (unknown page)</span>
              )}
              {' · '}
              {new Date(item.block.updatedAt).toLocaleString()}
            </div>
          </div>
        )
      })}
    </div>
  )
}
