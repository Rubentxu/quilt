import { useState } from 'react'
import { ChevronDown, ChevronRight, Link2, X, Check, FileText } from 'lucide-react'
import type { UnlinkedCandidate } from './unlinkedRefQueue'

export interface UnlinkedRefQueueProps {
  /** Current page name — used for grouping & "Link" tooltip. */
  pageName: string | null
  /** Candidates to display (owned by the parent — typically the hook). */
  queue: UnlinkedCandidate[]
  /** Loading state from the scan. */
  loading?: boolean
  /** Triggered by the "Link" button on a candidate row. */
  onLink: (candidate: UnlinkedCandidate) => void | Promise<void>
  /** Triggered by the "Dismiss" button on a candidate row. */
  onDismiss: (candidate: UnlinkedCandidate) => void
  /** Default collapse state for the section. Defaults to false. */
  defaultExpanded?: boolean
  /**
   * Optional per-block content override for displaying a richer
   * mention preview. The hook owns the queue; this lets the
   * BacklinksPanel pass the actual block content it already has in
   * memory (so we don't have to re-fetch for every row).
   */
  blockContentResolver?: (blockId: string) => string | undefined
}

/**
 * Compact panel listing the unlinked-reference candidates the user
 * hasn't actioned yet. Each row shows a 1-line preview of the
 * mention (derived from `mentionText` and the surrounding block
 * content when available), with "Link" and "Dismiss" buttons.
 *
 * The component is presentation-only: state (loading, queue,
 * async actions) is owned by the parent via the `useUnlinkedRefQueue`
 * hook. That keeps the panel test-friendly and re-usable in
 * contexts other than `BacklinksPanel` (e.g. a future global
 * "References" view).
 */
export function UnlinkedRefQueue({
  pageName,
  queue,
  loading = false,
  onLink,
  onDismiss,
  defaultExpanded = false,
  blockContentResolver,
}: UnlinkedRefQueueProps) {
  const [expanded, setExpanded] = useState(defaultExpanded)
  const [pendingId, setPendingId] = useState<string | null>(null)

  if (!pageName) return null

  const count = queue.length
  const keyOf = (c: UnlinkedCandidate) => `${c.blockId}:${c.position}`

  async function handleLink(c: UnlinkedCandidate) {
    setPendingId(keyOf(c))
    try {
      await onLink(c)
    } finally {
      setPendingId(null)
    }
  }

  return (
    <section data-testid="unlinked-ref-queue" style={{ marginTop: 'var(--space-4)' }}>
      {/* Header — mirrors the BacklinksPanel header pattern */}
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
        aria-controls="unlinked-ref-queue-content"
        data-testid="unlinked-ref-queue-header"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          marginBottom: expanded ? 'var(--space-3)' : 0,
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          background: 'transparent',
          border: 'none',
          padding: 0,
          cursor: 'pointer',
          width: '100%',
          textAlign: 'left',
        }}
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <FileText size={14} />
        <span>Unlinked Refs</span>
        <span
          data-testid="unlinked-ref-queue-count"
          style={{
            fontSize: '12px',
            fontWeight: 400,
            color: 'var(--color-text-muted)',
            marginLeft: 'auto',
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-pill)',
            padding: '0 8px',
            lineHeight: '18px',
            minWidth: '20px',
            textAlign: 'center',
          }}
        >
          {count}
        </span>
      </button>

      {expanded && (
        <div data-testid="unlinked-ref-queue-content">
          {loading && count === 0 && (
            <div
              style={{
                fontSize: '12px',
                color: 'var(--color-text-muted)',
                textAlign: 'center',
                padding: 'var(--space-3)',
              }}
            >
              Scanning for unlinked mentions...
            </div>
          )}

          {!loading && count === 0 && (
            <div
              data-testid="unlinked-ref-queue-empty"
              style={{
                padding: 'var(--space-3)',
                fontSize: '12px',
                color: 'var(--color-text-muted)',
                textAlign: 'center',
              }}
            >
              No unlinked references
            </div>
          )}

          {count > 0 && (
            <ul
              data-testid="unlinked-ref-queue-list"
              style={{ listStyle: 'none', padding: 0, margin: 0 }}
            >
              {queue.map((c) => {
                const key = keyOf(c)
                const content = blockContentResolver?.(c.blockId)
                const preview = buildPreview(c, content)
                const isPending = pendingId === key
                return (
                  <li
                    key={key}
                    data-testid="unlinked-ref-queue-item"
                    data-block-id={c.blockId}
                    data-position={c.position}
                    style={{
                      border: '1px solid var(--color-border)',
                      borderRadius: 'var(--radius-md)',
                      padding: 'var(--space-3)',
                      marginBottom: 'var(--space-2)',
                      background: 'var(--color-surface)',
                    }}
                  >
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'flex-start',
                        gap: 'var(--space-2)',
                        marginBottom: 'var(--space-2)',
                      }}
                    >
                      <Link2
                        size={11}
                        style={{
                          color: 'var(--color-text-muted)',
                          flexShrink: 0,
                          marginTop: '3px',
                        }}
                      />
                      <div
                        style={{
                          fontSize: '13px',
                          color: 'var(--color-text-primary)',
                          lineHeight: 1.4,
                          flex: 1,
                          wordBreak: 'break-word',
                        }}
                      >
                        {preview}
                      </div>
                    </div>
                    <div
                      style={{
                        display: 'flex',
                        gap: 'var(--space-2)',
                        justifyContent: 'flex-end',
                      }}
                    >
                      <button
                        type="button"
                        onClick={() => handleLink(c)}
                        disabled={isPending}
                        data-testid="unlinked-ref-queue-link"
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '4px',
                          padding: '4px 10px',
                          fontSize: '12px',
                          fontWeight: 500,
                          color: 'var(--color-surface)',
                          background: 'var(--color-accent, #2563eb)',
                          border: 'none',
                          borderRadius: 'var(--radius-sm)',
                          cursor: isPending ? 'wait' : 'pointer',
                          opacity: isPending ? 0.7 : 1,
                        }}
                        aria-label={`Link ${c.pageName}`}
                      >
                        <Check size={11} />
                        {isPending ? 'Linking...' : 'Link'}
                      </button>
                      <button
                        type="button"
                        onClick={() => onDismiss(c)}
                        data-testid="unlinked-ref-queue-dismiss"
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '4px',
                          padding: '4px 10px',
                          fontSize: '12px',
                          fontWeight: 500,
                          color: 'var(--color-text-secondary)',
                          background: 'transparent',
                          border: '1px solid var(--color-border)',
                          borderRadius: 'var(--radius-sm)',
                          cursor: 'pointer',
                        }}
                        aria-label={`Dismiss mention of ${c.pageName}`}
                      >
                        <X size={11} />
                        Dismiss
                      </button>
                    </div>
                  </li>
                )
              })}
            </ul>
          )}
        </div>
      )}
    </section>
  )
}

/**
 * Build a 1-line preview around the mention. If the parent gave us
 * the full block content, we slice ~30 chars on either side of the
 * mention and wrap the mention in a `<mark>` for visual emphasis.
 * Otherwise we fall back to a plain stub that shows the mention
 * text in context.
 */
function buildPreview(c: UnlinkedCandidate, content: string | undefined): React.ReactNode {
  if (content) {
    const ctx = 30
    const start = Math.max(0, c.position - ctx)
    const end = Math.min(content.length, c.position + c.mentionText.length + ctx)
    const before = content.slice(start, c.position)
    const match = content.slice(c.position, c.position + c.mentionText.length)
    const after = content.slice(c.position + c.mentionText.length, end)
    const prefix = start > 0 ? '…' : ''
    const suffix = end < content.length ? '…' : ''
    return (
      <>
        {prefix}
        {before}
        <mark
          style={{
            background: 'var(--color-accent-subtle, #dbeafe)',
            color: 'inherit',
            padding: '0 1px',
            borderRadius: '2px',
          }}
        >
          {match}
        </mark>
        {after}
        {suffix}
      </>
    )
  }
  return <span>…contains “{c.mentionText}”…</span>
}
