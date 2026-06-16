// ─── MorningBriefing ───────────────────────────────────────────────
//
// CG-1: Morning Briefing end-to-end.
// Displays a daily snapshot of the knowledge graph including:
// - Today's agenda (recent blocks)
// - Decay alerts (stale blocks needing attention)
// - Serendipity highlights (unexpected connections)
//
// Per ADR-0001, Quilt does NOT integrate AI/LLM providers.
// MorningBriefing aggregates structural data from the graph.

import { useCallback, useEffect, useState } from 'react'
import { Sun, AlertTriangle, Sparkles, Loader2 } from 'lucide-react'
import { api } from '@core/api-client'
import type { MorningBriefingDto, AgendaItem, DecayAlert, SerendipityHighlight } from '@shared/types/api'

interface MorningBriefingProps {
  /** Called when the user clicks on an agenda item or decay alert. */
  onNavigate?: (blockId: string, pageName: string) => void
}

// ─── Section: Agenda ──────────────────────────────────────────────

function AgendaSection({
  items,
  onNavigate,
}: {
  items: AgendaItem[]
  onNavigate?: (blockId: string, pageName: string) => void
}) {
  if (items.length === 0) {
    return (
      <div data-testid="morning-briefing-agenda-empty" style={emptyStyle}>
        No activity today yet
      </div>
    )
  }
  return (
    <ul data-testid="morning-briefing-agenda-list" style={{ listStyle: 'none', margin: 0, padding: 0 }} role="list">
      {items.map((item) => (
        <li
          key={item.blockId}
          data-testid={`morning-briefing-agenda-item-${item.blockId}`}
          role="listitem"
          style={{
            padding: 'var(--space-2)',
            borderBottom: '1px solid var(--color-border)',
            fontSize: '12px',
            cursor: onNavigate ? 'pointer' : 'default',
          }}
          onClick={() => onNavigate?.(item.blockId, item.pageName)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              onNavigate?.(item.blockId, item.pageName)
            }
          }}
          tabIndex={onNavigate ? 0 : undefined}
          aria-label={`Agenda item: ${item.contentPreview.slice(0, 60)} on ${item.pageName}`}
        >
          <div style={{ color: 'var(--color-accent)', fontWeight: 500, fontSize: '11px', marginBottom: '2px' }}>
            {item.pageName}
          </div>
          <div style={{ color: 'var(--color-text-secondary)', lineHeight: 1.4 }}>
            {item.contentPreview.length > 100
              ? item.contentPreview.slice(0, 100) + '…'
              : item.contentPreview || '(empty block)'}
          </div>
          {item.hasChildren && (
            <div style={{ color: 'var(--color-text-muted)', fontSize: '10px', marginTop: '2px' }}>
              Has children
            </div>
          )}
        </li>
      ))}
    </ul>
  )
}

// ─── Section: Decay Alerts ────────────────────────────────────────

function severityColor(severity: string): string {
  if (severity === 'high') return 'var(--color-danger, #c0392b)'
  if (severity === 'medium') return 'var(--color-warning, #e67e22)'
  return 'var(--color-text-muted)'
}

function DecaySection({
  alerts,
  onNavigate,
}: {
  alerts: DecayAlert[]
  onNavigate?: (blockId: string, pageName: string) => void
}) {
  if (alerts.length === 0) {
    return (
      <div data-testid="morning-briefing-decay-empty" style={emptyStyle}>
        No decay alerts — everything looks healthy
      </div>
    )
  }
  return (
    <ul data-testid="morning-briefing-decay-list" style={{ listStyle: 'none', margin: 0, padding: 0 }} role="list">
      {alerts.map((alert) => (
        <li
          key={alert.blockId}
          data-testid={`morning-briefing-decay-item-${alert.blockId}`}
          role="listitem"
          style={{
            padding: 'var(--space-2)',
            borderBottom: '1px solid var(--color-border)',
            fontSize: '12px',
            cursor: onNavigate ? 'pointer' : 'default',
          }}
          onClick={() => onNavigate?.(alert.blockId, alert.pageName)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              onNavigate?.(alert.blockId, alert.pageName)
            }
          }}
          tabIndex={onNavigate ? 0 : undefined}
          aria-label={`Decay alert: ${alert.contentPreview.slice(0, 60)} — ${alert.reason}`}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
            <AlertTriangle size={11} color={severityColor(alert.severity)} />
            <span style={{ color: severityColor(alert.severity), fontWeight: 600, fontSize: '11px' }}>
              {alert.severity.toUpperCase()}
            </span>
            <span style={{ color: 'var(--color-text-muted)', fontSize: '10px' }}>
              {alert.daysSinceUpdate}d ago
            </span>
          </div>
          <div style={{ color: 'var(--color-text-secondary)', lineHeight: 1.4 }}>
            {alert.contentPreview.length > 100
              ? alert.contentPreview.slice(0, 100) + '…'
              : alert.contentPreview || '(empty block)'}
          </div>
          <div style={{ color: 'var(--color-text-muted)', fontSize: '10px', marginTop: '2px' }}>
            {alert.pageName}
          </div>
        </li>
      ))}
    </ul>
  )
}

// ─── Section: Serendipity Highlights ─────────────────────────────

function SerendipitySection({ highlights }: { highlights: SerendipityHighlight[] }) {
  if (highlights.length === 0) {
    return (
      <div data-testid="morning-briefing-serendipity-empty" style={emptyStyle}>
        No serendipity highlights yet
      </div>
    )
  }
  return (
    <ul data-testid="morning-briefing-serendipity-list" style={{ listStyle: 'none', margin: 0, padding: 0 }} role="list">
      {highlights.map((h, i) => (
        <li
          key={`${h.blockAId}-${h.blockBId}-${i}`}
          data-testid={`morning-briefing-serendipity-item-${i}`}
          role="listitem"
          style={{
            padding: 'var(--space-2)',
            borderBottom: '1px solid var(--color-border)',
            fontSize: '12px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '4px' }}>
            <Sparkles size={11} color="var(--color-accent)" />
            <span style={{ color: 'var(--color-accent)', fontWeight: 600, fontSize: '11px' }}>
              {Math.round(h.confidence * 100)}% match
            </span>
          </div>
          <div style={{ color: 'var(--color-text-secondary)', lineHeight: 1.4, fontStyle: 'italic' }}>
            {h.explanation}
          </div>
        </li>
      ))}
    </ul>
  )
}

// ─── Shared styles ────────────────────────────────────────────────

const emptyStyle: React.CSSProperties = {
  padding: 'var(--space-2)',
  color: 'var(--color-text-muted)',
  fontSize: '12px',
  fontStyle: 'italic',
}

const sectionHeaderStyle: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 600,
  color: 'var(--color-text-muted)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
  padding: 'var(--space-1) var(--space-2)',
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
}

// ─── Main component ────────────────────────────────────────────────

export function MorningBriefing({ onNavigate }: MorningBriefingProps) {
  const [data, setData] = useState<MorningBriefingDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      const result = await api.getMorningBriefing()
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load morning briefing')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    void load(false)
  }, [load])

  return (
    <div
      data-testid="morning-briefing"
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
      role="region"
      aria-label="Morning Briefing"
    >
      {/* Header */}
      <div
        style={{
          padding: 'var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Sun size={16} color="var(--color-accent)" />
          <span style={{ fontWeight: 600, fontSize: '14px' }}>Morning Briefing</span>
          {data && (
            <span style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}>
              {data.daysSinceLastJournal === 0
                ? 'Journal updated today'
                : `Last journal ${data.daysSinceLastJournal}d ago`}
            </span>
          )}
        </div>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh morning briefing"
          data-testid="morning-briefing-refresh"
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
          <Loader2
            size={14}
            style={{
              animation: refreshing ? 'spin 1s linear infinite' : 'none',
            }}
          />
        </button>
      </div>

      {/* Loading state */}
      {loading && (
        <div data-testid="morning-briefing-loading" style={{ padding: 'var(--space-4)', textAlign: 'center' }}>
          <Loader2 size={16} color="var(--color-text-muted)" style={{ animation: 'spin 1s linear infinite' }} />
          <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', marginTop: 'var(--space-2)' }}>
            Loading briefing…
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div
          data-testid="morning-briefing-error"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-danger, #c0392b)',
            fontSize: '12px',
          }}
        >
          {error}
        </div>
      )}

      {/* Content */}
      {!loading && !error && data && (
        <div>
          {/* Today's Agenda */}
          <section aria-labelledby="morning-briefing-agenda-label">
            <div
              id="morning-briefing-agenda-label"
              data-testid="morning-briefing-agenda-header"
              style={sectionHeaderStyle}
            >
              <Sun size={12} /> Today's Agenda
              <span style={{ marginLeft: 'auto', fontWeight: 400, textTransform: 'none', letterSpacing: 0 }}>
                {data.agendaItems.length} items
              </span>
            </div>
            <AgendaSection items={data.agendaItems} onNavigate={onNavigate} />
          </section>

          {/* Decay Alerts */}
          <section aria-labelledby="morning-briefing-decay-label" style={{ borderTop: '1px solid var(--color-border)' }}>
            <div
              id="morning-briefing-decay-label"
              data-testid="morning-briefing-decay-header"
              style={sectionHeaderStyle}
            >
              <AlertTriangle size={12} /> Decay Alerts
              <span style={{ marginLeft: 'auto', fontWeight: 400, textTransform: 'none', letterSpacing: 0 }}>
                {data.decayAlerts.length} alerts
              </span>
            </div>
            <DecaySection alerts={data.decayAlerts} onNavigate={onNavigate} />
          </section>

          {/* Serendipity Highlights */}
          <section aria-labelledby="morning-briefing-serendipity-label" style={{ borderTop: '1px solid var(--color-border)' }}>
            <div
              id="morning-briefing-serendipity-label"
              data-testid="morning-briefing-serendipity-header"
              style={sectionHeaderStyle}
            >
              <Sparkles size={12} /> Serendipity Highlights
              <span style={{ marginLeft: 'auto', fontWeight: 400, textTransform: 'none', letterSpacing: 0 }}>
                {data.serendipityHighlights.length} found
              </span>
            </div>
            <SerendipitySection highlights={data.serendipityHighlights} />
          </section>

          {/* Generated at footer */}
          <div
            data-testid="morning-briefing-footer"
            style={{
              padding: 'var(--space-2)',
              borderTop: '1px solid var(--color-border)',
              color: 'var(--color-text-muted)',
              fontSize: '10px',
              textAlign: 'center',
            }}
          >
            Generated {new Date(data.generatedAt).toLocaleString()}
          </div>
        </div>
      )}
    </div>
  )
}
