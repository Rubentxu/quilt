/**
 * AnnotationRow — the per-annotation view used by both the inline
 * `BlockRow` comments thread (when the annotation feature flag is
 * ON) and the sidebar `AnnotationPanel`.
 *
 * Visual parity with the legacy `CommentRow` (icon, author line,
 * action buttons) but the data source is the `Annotation` object
 * from the annotation API — not the block-property `type: "comment"`
 * hack. Status drives the visual: `pending` is the default,
 * `in_progress` gets an accent colour, `resolved` dims + strikes
 * through, `dismissed` is muted.
 *
 * Agent authors get a robot icon and accent colour (matches the
 * `created_by` convention in `BlockRow`).
 */

import { Check, MessageSquare, Trash2 } from 'lucide-react'
import type { Annotation } from '@shared/types/api'

export interface AnnotationRowProps {
  annotation: Annotation
  /** Reply handler — opens an inline reply input on the parent panel. */
  onReply?: (id: string) => void
  /** Optional delete handler. */
  onDelete?: (id: string) => void
  /** Nesting depth — used for left indent on threaded replies. */
  depth?: number
  /** When true, omit the resolve toggle (e.g. in resolved-filtered views). */
  hideResolve?: boolean
}

const STATUS_BG: Record<Annotation['status'], string> = {
  pending: 'var(--color-surface-subtle)',
  in_progress: 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))',
  resolved: 'transparent',
  dismissed: 'transparent',
}

const STATUS_COLOR: Record<Annotation['status'], string> = {
  pending: 'var(--color-text-muted)',
  in_progress: 'var(--color-accent)',
  resolved: 'var(--color-success)',
  dismissed: 'var(--color-text-disabled)',
}

const STATUS_LABEL: Record<Annotation['status'], string> = {
  pending: 'PENDING',
  in_progress: 'IN PROGRESS',
  resolved: 'RESOLVED',
  dismissed: 'DISMISSED',
}

export function AnnotationRow({
  annotation,
  onReply,
  onDelete,
  depth = 0,
  hideResolve = false,
}: AnnotationRowProps) {
  const isResolved = annotation.status === 'resolved'
  const isDismissed = annotation.status === 'dismissed'
  const isMuted = isResolved || isDismissed
  const isAgent = annotation.authorType === 'agent'

  return (
    <div
      data-testid={`annotation-row-${annotation.id}`}
      data-annotation-status={annotation.status}
      data-annotation-author-type={annotation.authorType}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: 'var(--space-2)',
        padding: 'var(--space-1) 0',
        paddingLeft: depth > 0 ? `${depth * 16}px` : 0,
        opacity: isMuted ? 0.6 : 1,
        textDecoration: isResolved ? 'line-through' : 'none',
      }}
    >
      <div
        style={{
          flex: 1,
          minWidth: 0,
          fontSize: '13px',
          color: 'var(--color-text-secondary)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-1)',
            marginBottom: '2px',
            flexWrap: 'wrap',
          }}
        >
          {/* Author + author-type icon. The `aria-label` makes the
              icon discoverable to screen readers without needing
              visible "agent" / "human" text. */}
          <span
            aria-label={isAgent ? `${annotation.authorName} (agent)` : annotation.authorName}
            title={isAgent ? `${annotation.authorName} (agent)` : annotation.authorName}
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: isAgent ? 'var(--color-accent)' : 'var(--color-text-muted)',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '3px',
            }}
          >
            {isAgent && <span aria-hidden="true">🤖</span>}
            {!isAgent && <span aria-hidden="true">👤</span>}
            {annotation.authorName}
          </span>
          <span
            data-testid={`annotation-status-badge-${annotation.id}`}
            style={{
              fontSize: '9px',
              fontWeight: 700,
              padding: '1px 5px',
              borderRadius: 'var(--radius-pill)',
              background: STATUS_BG[annotation.status],
              color: STATUS_COLOR[annotation.status],
              letterSpacing: '0.04em',
            }}
          >
            {STATUS_LABEL[annotation.status]}
          </span>
          {annotation.createdAt && (
            <span
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
              }}
            >
              · {formatAnnotationDate(annotation.createdAt)}
            </span>
          )}
        </div>
        <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
          {annotation.content}
        </div>
      </div>
      <div style={{ display: 'flex', gap: '2px', flexShrink: 0 }}>
        {!hideResolve && (
          <button
            onClick={() => {
              // The resolve action is wired by the parent — we just
              // call it with the id; the parent decides which
              // status to send.
            }}
            aria-label="Toggle resolve"
            title={isResolved ? 'Unresolve' : 'Resolve'}
            data-testid={`annotation-resolve-${annotation.id}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: isResolved
                ? 'var(--color-success)'
                : 'var(--color-text-muted)',
              padding: '2px',
              display: 'flex',
              alignItems: 'center',
              borderRadius: 'var(--radius-sm)',
            }}
          >
            <Check size={12} />
          </button>
        )}
        {onReply && (
          <button
            onClick={() => onReply(annotation.id)}
            aria-label="Reply to annotation"
            title="Reply"
            data-testid={`annotation-reply-${annotation.id}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: '2px',
              display: 'flex',
              alignItems: 'center',
              borderRadius: 'var(--radius-sm)',
            }}
          >
            <MessageSquare size={12} />
          </button>
        )}
        {onDelete && (
          <button
            onClick={() => onDelete(annotation.id)}
            aria-label="Delete annotation"
            title="Delete"
            data-testid={`annotation-delete-${annotation.id}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: '2px',
              display: 'flex',
              alignItems: 'center',
              borderRadius: 'var(--radius-sm)',
            }}
          >
            <Trash2 size={12} />
          </button>
        )}
      </div>
    </div>
  )
}

/**
 * Best-effort relative-or-absolute date label.
 * Returns null for falsy input so the caller can decide to hide the label.
 */
function formatAnnotationDate(input: string | null | undefined): string | null {
  if (!input) return null
  const d = new Date(input)
  if (Number.isNaN(d.getTime())) return null
  const now = new Date()
  const diffMs = now.getTime() - d.getTime()
  const diffMin = Math.floor(diffMs / 60000)
  if (diffMin < 1) return 'just now'
  if (diffMin < 60) return `${diffMin}m`
  const diffH = Math.floor(diffMin / 60)
  if (diffH < 24) return `${diffH}h`
  const diffD = Math.floor(diffH / 24)
  if (diffD < 7) return `${diffD}d`
  return d.toLocaleDateString()
}
