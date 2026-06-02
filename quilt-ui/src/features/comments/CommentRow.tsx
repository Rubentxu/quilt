// ──── CommentRow ─────────────────────────────────────────────────
// Single comment rendered inside a block's comment thread.
// Supports resolve, reply, and delete actions.
//
// Comments are regular blocks with `type: "comment"` property. The
// `created_by` and `created_at` properties carry authorship info,
// `resolved` toggles the resolved state.

import { Check, MessageSquare, Trash2 } from 'lucide-react'
import type { Block } from '@shared/types/api'
import { getBlockProperty } from '@shared/utils/blockProperties'

export interface CommentRowProps {
  comment: Block
  /** Toggle the `resolved` property on the comment block. */
  onResolve: (id: string) => void
  /** Add a reply — a new comment child of this one. */
  onReply: (id: string) => void
  /** Optional delete handler. */
  onDelete?: (id: string) => void
  /** Nesting depth — used for left indent. */
  depth?: number
}

export function CommentRow({
  comment,
  onResolve,
  onReply,
  onDelete,
  depth = 0,
}: CommentRowProps) {
  const isResolved =
    String(getBlockProperty(comment.properties, 'resolved') ?? 'false') ===
    'true'
  const createdBy = getBlockProperty(comment.properties, 'created_by')
  const createdAt = getBlockProperty(comment.properties, 'created_at')

  const author = (createdBy ?? 'anonymous') as string
  const dateLabel = formatCommentDate(createdAt as string | null | undefined)

  return (
    <div
      data-testid={`comment-row-${comment.id}`}
      data-comment-resolved={isResolved ? 'true' : 'false'}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: 'var(--space-2)',
        padding: 'var(--space-1) 0',
        paddingLeft: depth > 0 ? `${depth * 16}px` : 0,
        opacity: isResolved ? 0.5 : 1,
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
          }}
        >
          <span
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
            }}
          >
            {author}
          </span>
          {dateLabel && (
            <span
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
              }}
            >
              · {dateLabel}
            </span>
          )}
        </div>
        <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
          {comment.content}
        </div>
      </div>
      <div style={{ display: 'flex', gap: '2px', flexShrink: 0 }}>
        <button
          onClick={() => onResolve(comment.id)}
          aria-label={isResolved ? 'Unresolve comment' : 'Resolve comment'}
          title={isResolved ? 'Unresolve' : 'Resolve'}
          data-testid={`comment-resolve-${comment.id}`}
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
        <button
          onClick={() => onReply(comment.id)}
          aria-label="Reply to comment"
          title="Reply"
          data-testid={`comment-reply-${comment.id}`}
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
        {onDelete && (
          <button
            onClick={() => onDelete(comment.id)}
            aria-label="Delete comment"
            title="Delete"
            data-testid={`comment-delete-${comment.id}`}
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
 * Best-effort relative-or-absolute date label for a comment.
 * Returns null for falsy input so the caller can decide to hide the label.
 */
function formatCommentDate(input: string | null | undefined): string | null {
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
