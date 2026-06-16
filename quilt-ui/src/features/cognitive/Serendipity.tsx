// ─── Serendipity ───────────────────────────────────────────────────────────────
//
// CG-3: Serendipity UI end-to-end.
// Standalone panel that surfaces unexpected connections between blocks
// discovered by the connection engine.
//
// Per ADR-0001, Quilt does NOT integrate AI/LLM providers.
// Serendipity detection is purely structural (shared refs + temporal proximity).

import { useCallback, useEffect, useState } from 'react'
import { Sparkles, Loader2, ExternalLink, X, Star } from 'lucide-react'
import { api } from '@core/api-client'
import type { SerendipityHighlight, SerendipityResponseDto } from '@shared/types/api'

interface SerendipityProps {
  /** Called when the user wants to open a block. */
  onNavigate?: (blockId: string, pageName: string | null) => void
}

// ─── Helpers ───────────────────────────────────────────────────────────────

function confidenceColor(confidence: number): string {
  if (confidence >= 0.7) return 'var(--color-accent)'
  if (confidence >= 0.5) return 'var(--color-warning, #e67e22)'
  return 'var(--color-text-muted)'
}

function confidenceLabel(confidence: number): string {
  return `${Math.round(confidence * 100)}% match`
}

// ─── Empty state ─────────────────────────────────────────────────────────

function SerendipityEmpty() {
  return (
    <div
      data-testid="serendipity-empty"
      style={{
        padding: 'var(--space-2)',
        color: 'var(--color-text-muted)',
        fontSize: '12px',
        fontStyle: 'italic',
      }}
    >
      No unexpected connections found — keep writing!
    </div>
  )
}

// ─── Highlight item ───────────────────────────────────────────────────────

interface HighlightItemProps {
  highlight: SerendipityHighlight
  onNavigate?: (blockId: string, pageName: string | null) => void
  onAccept?: (highlight: SerendipityHighlight) => void
  onIgnore?: (highlight: SerendipityHighlight) => void
}

function HighlightItem({ highlight, onNavigate, onAccept, onIgnore }: HighlightItemProps) {
  const handleOpenA = () => onNavigate?.(highlight.blockAId, null)
  const handleOpenB = () => onNavigate?.(highlight.blockBId, null)
  const handleOpenBoth = () => {
    onNavigate?.(highlight.blockAId, null)
    onNavigate?.(highlight.blockBId, null)
  }

  return (
    <li
      data-testid={`serendipity-item-${highlight.blockAId}`}
      role="listitem"
      style={{
        padding: 'var(--space-2)',
        borderBottom: '1px solid var(--color-border)',
        fontSize: '12px',
      }}
    >
      {/* Confidence badge */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
          marginBottom: '6px',
        }}
      >
        <Sparkles size={11} color={confidenceColor(highlight.confidence)} />
        <span
          style={{
            color: confidenceColor(highlight.confidence),
            fontWeight: 600,
            fontSize: '11px',
          }}
        >
          {confidenceLabel(highlight.confidence)}
        </span>
      </div>

      {/* Block previews */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', marginBottom: '6px' }}>
        <div
          style={{
            color: 'var(--color-text-secondary)',
            lineHeight: 1.4,
            background: 'var(--color-surface-raised)',
            padding: '4px 6px',
            borderRadius: 'var(--radius-sm)',
            cursor: onNavigate ? 'pointer' : 'default',
          }}
          onClick={handleOpenA}
          onKeyDown={(e) => {
            if ((e.key === 'Enter' || e.key === ' ') && onNavigate) {
              e.preventDefault()
              handleOpenA()
            }
          }}
          tabIndex={onNavigate ? 0 : undefined}
          aria-label={`Open block: ${highlight.blockAPreview.slice(0, 40)}`}
        >
          {highlight.blockAPreview.length > 80
            ? highlight.blockAPreview.slice(0, 80) + '…'
            : highlight.blockAPreview || '(empty block)'}
        </div>
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: '10px',
            textAlign: 'center',
          }}
        >
          ↕ connected via
        </div>
        <div
          style={{
            color: 'var(--color-text-secondary)',
            lineHeight: 1.4,
            background: 'var(--color-surface-raised)',
            padding: '4px 6px',
            borderRadius: 'var(--radius-sm)',
            cursor: onNavigate ? 'pointer' : 'default',
          }}
          onClick={handleOpenB}
          onKeyDown={(e) => {
            if ((e.key === 'Enter' || e.key === ' ') && onNavigate) {
              e.preventDefault()
              handleOpenB()
            }
          }}
          tabIndex={onNavigate ? 0 : undefined}
          aria-label={`Open block: ${highlight.blockBPreview.slice(0, 40)}`}
        >
          {highlight.blockBPreview.length > 80
            ? highlight.blockBPreview.slice(0, 80) + '…'
            : highlight.blockBPreview || '(empty block)'}
        </div>
      </div>

      {/* Explanation */}
      <div
        style={{
          color: 'var(--color-text-muted)',
          fontSize: '11px',
          fontStyle: 'italic',
          marginBottom: '6px',
        }}
      >
        {highlight.explanation}
      </div>

      {/* Action buttons */}
      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
        <button
          onClick={handleOpenBoth}
          aria-label="Open both connected blocks"
          data-testid={`serendipity-open-both-${highlight.blockAId}`}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '3px',
            background: 'none',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            padding: '2px 6px',
            fontSize: '10px',
            cursor: 'pointer',
            color: 'var(--color-text-secondary)',
          }}
        >
          <ExternalLink size={10} />
          Open both
        </button>
        {onAccept && (
          <button
            onClick={() => onAccept(highlight)}
            aria-label="Accept connection"
            data-testid={`serendipity-accept-${highlight.blockAId}`}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '3px',
              background: 'none',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: '2px 6px',
              fontSize: '10px',
              cursor: 'pointer',
              color: 'var(--color-accent)',
            }}
          >
            <Star size={10} />
            Accept
          </button>
        )}
        {onIgnore && (
          <button
            onClick={() => onIgnore(highlight)}
            aria-label="Ignore this connection"
            data-testid={`serendipity-ignore-${highlight.blockAId}`}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '3px',
              background: 'none',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: '2px 6px',
              fontSize: '10px',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
            }}
          >
            <X size={10} />
            Ignore
          </button>
        )}
      </div>
    </li>
  )
}

// ─── Main component ───────────────────────────────────────────────────────

export function Serendipity({ onNavigate }: SerendipityProps) {
  const [data, setData] = useState<SerendipityResponseDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (isRefresh: boolean) => {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    setError(null)
    try {
      const result = await api.getSerendipity()
      setData(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load serendipity highlights')
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    void load(false)
  }, [load])

  // Accept action — for now just removes from local state (server integration TBD)
  const handleAccept = useCallback((highlight: SerendipityHighlight) => {
    setData((prev) => {
      if (!prev) return prev
      return {
        ...prev,
        highlights: prev.highlights.filter(
          (h) => !(h.blockAId === highlight.blockAId && h.blockBId === highlight.blockBId),
        ),
      }
    })
  }, [])

  // Ignore action — same as accept for now
  const handleIgnore = useCallback((highlight: SerendipityHighlight) => {
    setData((prev) => {
      if (!prev) return prev
      return {
        ...prev,
        highlights: prev.highlights.filter(
          (h) => !(h.blockAId === highlight.blockAId && h.blockBId === highlight.blockBId),
        ),
      }
    })
  }, [])

  return (
    <div
      data-testid="serendipity"
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        overflow: 'hidden',
      }}
      role="region"
      aria-label="Serendipity Monitor"
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
          <Sparkles size={16} color="var(--color-accent)" />
          <span style={{ fontWeight: 600, fontSize: '14px' }}>Serendipity</span>
          {data && (
            <span
              data-testid="serendipity-count"
              style={{ color: 'var(--color-text-muted)', fontSize: '11px' }}
            >
              {data.total} found
            </span>
          )}
        </div>
        <button
          onClick={() => void load(true)}
          disabled={refreshing}
          aria-label="Refresh serendipity highlights"
          data-testid="serendipity-refresh"
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
        <div
          data-testid="serendipity-loading"
          style={{ padding: 'var(--space-4)', textAlign: 'center' }}
        >
          <Loader2
            size={16}
            color="var(--color-text-muted)"
            style={{ animation: 'spin 1s linear infinite' }}
          />
          <div
            style={{
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              marginTop: 'var(--space-2)',
            }}
          >
            Finding unexpected connections…
          </div>
        </div>
      )}

      {/* Error state */}
      {error && !loading && (
        <div
          data-testid="serendipity-error"
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
          {data.highlights.length === 0 ? (
            <SerendipityEmpty />
          ) : (
            <ul
              data-testid="serendipity-list"
              style={{ listStyle: 'none', margin: 0, padding: 0 }}
              role="list"
            >
              {data.highlights.map((highlight) => (
                <HighlightItem
                  key={`${highlight.blockAId}-${highlight.blockBId}`}
                  highlight={highlight}
                  onNavigate={onNavigate}
                  onAccept={handleAccept}
                  onIgnore={handleIgnore}
                />
              ))}
            </ul>
          )}

          {/* Generated at footer */}
          <div
            data-testid="serendipity-footer"
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
