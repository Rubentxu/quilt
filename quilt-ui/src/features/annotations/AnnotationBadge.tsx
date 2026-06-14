/**
 * AnnotationBadge — small count pill rendered on a block row when
 * the block has 1+ non-resolved annotation. Mirrors the
 * `BlockRow` `MARKER_STYLES` / `PRIORITY_STYLES` shape (small,
 * pill radius, label-md) but tinted accent to read as
 * "annotation" not "marker".
 *
 * The count is "pending + in_progress" (matching the sidebar badge
 * in `spec-annotation-panel` — resolved annotations are still on the
 * block but don't surface in the count).
 */

import { MessageCircle } from 'lucide-react'

export interface AnnotationBadgeProps {
  /** Pending + in-progress count. The badge is hidden when this is 0. */
  count: number
  /** Click handler — opens the annotation thread below the block. */
  onClick?: () => void
  /** Optional testid override. */
  testId?: string
}

export function AnnotationBadge({ count, onClick, testId }: AnnotationBadgeProps) {
  if (count <= 0) return null
  return (
    <span
      data-testid={testId ?? 'annotation-badge'}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={
        onClick
          ? e => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                onClick()
              }
            }
          : undefined
      }
      title={`${count} pending annotation${count === 1 ? '' : 's'}`}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '3px',
        flexShrink: 0,
        alignSelf: 'center',
        fontSize: '10px',
        fontWeight: 600,
        padding: '1px 6px',
        borderRadius: 'var(--radius-pill)',
        background: 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))',
        color: 'var(--color-accent)',
        cursor: onClick ? 'pointer' : 'default',
        lineHeight: 1.4,
        letterSpacing: '0.01em',
      }}
    >
      <MessageCircle size={10} aria-hidden="true" />
      {count}
    </span>
  )
}
